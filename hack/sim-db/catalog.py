#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' results catalog: every sim result the machine knows of, in one
DuckLake, so that any question is one SQL statement over it.

  hack/sim-db/simdb.py index [--workers N] [--quiet]
  hack/sim-db/simdb.py fetch --exp E [--cells C,...] [--arms A,...] [--tables reads,forecast]
  hack/sim-db/simdb.py lake QUERY [--json]

The catalog is a DuckLake (the `ducklake` extension, EXTENSIONS pins its
build for the pinned DuckDB CLI) whose metadata is SQLite,
<cache root>/lake/catalog.sqlite (the cache root is simdb.py's,
`SIM_DB_ROOT` or the main checkout's target/sim-db), and whose data are
Parquet files under lake/data/. One writer (`index`) and any number of
readers coexist: a reader attaches it read-only,

  LOAD ducklake; LOAD sqlite;
  ATTACH 'ducklake:sqlite:<lake>/catalog.sqlite' AS lake (READ_ONLY);
  USE lake;

and each query sees one snapshot, whatever the indexer is writing.

`index` brings the catalog up to date with the cache (simdb.py's: the
suite's and the farm's flushes), the suite's and the farm's indexes, the
farm's costs and S3 ledger, and git; it does only what changed since its
last run, is idempotent, runs under lake/index.lock, and does nothing but
look when nothing changed. A cache source's sim tables are hard links of
the cache's own Parquet files, registered with the lake
(ducklake_add_data_files), so they cost no disk while the cache holds
them; a file whose column types differ from the lake table's (a column
new to the lake takes the type most of its files give it) is rewritten,
cast. A source gone from the cache (simdb.py gc) leaves the catalog at
the next index, and its files with it. The rows, the verdicts and the
metadata tables are written by the lake, and its small files are merged
at the end of each index (ducklake_merge_adjacent_files); snapshots older
than 15 minutes, and the files only they read, go then too. After each
write the tables' statistics are marked as holding NULLs (null_stats).
The verdicts are simdb.py's derived tables (simdb.derive, analyze.py's
verdict and run_metrics), made for every source that lacks them, several
sources at a time (`--workers`).

`fetch` copies what S3 still holds of an experiment's jobs into the cache
(simdb.py's pull, and with `--tables` the bulky sample tables of the jobs
picked by `--cells` and `--arms`), indexes them, and prints what S3
charged for it, from the farm's S3 ledger. `lake` runs a query over the
catalog, read-only.

The tables (`table_catalog` holds this, by table, for the console and
for questions in plain language; `metric_catalog` holds each column and
metric, `event_kinds` each event):

  experiments       one an experiment: a suite farm run (exp_id its name,
                    s7ae7e), or a local suite invocation (exp_id its cache
                    source, suite/<worktree>/<invocation>)
  experiment_cells  one an experiment's cell (exp_id, cell)
  experiment_jobs   an experiment's jobs: (exp_id, cell, arm_label, seed)
                    names a job `k`; a job the farm reused counts in each
                    experiment it was reused for
  exp_runs          experiment_jobs with their jobs' columns
  jobs              one a job `k`: its sim, test, seed, commit, binary,
                    env (and the knobs promoted to columns), the source
                    its tables come from (`src`)
  commits           the commits the jobs and experiments name, from git
  costs, s3         the farm's cost lines and its S3 ledger, priced
  availability      what the catalog holds of each job, table by table,
                    and what S3 still could give it (`fetch`)
  rows              each job's row, a column a field
  scalars           the rows' numbers, long form (name, value)
  seconds, viewer_seconds, viewers, events, bins, node_bins, reads,
  stalls, parents, forecast, and any table a sim writes
                    the sims' tables, normalized: viewers `node` in both
                    sims, `churned` BOOLEAN, `arrive_ms` NULL for a viewer
                    there from the start; viewer_seconds without the
                    seconds a viewer wasn't there; events' `detail` also as
                    a MAP where it is key=value; -1 for none or no reading
                    NULL
  verdicts, verdict_seconds, episodes, derived_events
                    analyze.py's verdicts, of the current verdict_version
                    (catalog_info); <table>_all holds every version kept
  metric_defs, job_metrics, job_runs, t975
                    suite.py's METRICS (each one's SQL over a job's row,
                    verdict and run), every job's value of each, each
                    source's copy of a job with its own host's numbers, and
                    the t table the intervals use
  scoring.sql's macros
                    suite.py's scoring (scoring_sql), one implementation:
                    effects, collapse_test, traces, arm_cells and
                    arm_score score one experiment's arm against
                    another's, cell by cell, as `suite.py score` prints
                    them (the *_of macros take any two lists of runs);
                    structural(exp) gives the cells every arm collapsed on
                    at every seed, dynamics(exp) each arm and cell's lag by
                    phase, spikes past the playout delays, stalls and
                    spells. table_catalog describes each (kind 'macro' or
                    'table macro', keys its parameters)

Every table carries a job's key `k`, and the result tables and
experiment_jobs its source `src` too: a job's tables come from one cache
source, jobs.src (simdb.py's source_ranks: the one holding the most of
them, then the later flush, then the later seq), so a result table holds
one copy of a job, and any two join USING (k, src), as simdb.py's views
do. USING (k) joins the same rows, but USING (k, src) lets the source
prune the files a big table (reads, bins) reads: an experiment's reads
are `experiment_jobs e JOIN reads USING (k, src) WHERE e.exp_id = ...`.
`JOIN experiment_jobs USING (k, src)` gives a job's experiments, cells,
arms and seeds. Beside them the tables join on their own columns: reads,
viewer_seconds and stalls to viewers USING (k, src, viewer); events,
node_bins, parents and forecast to viewers USING (k, src, node); seconds
to verdict_seconds USING (k, src, sec).

  SELECT e.cell, e.arm_label, count(*) AS runs, sum(v.collapsed::INT) AS collapsed
  FROM experiment_jobs e JOIN verdicts v USING (k, src)
  WHERE e.exp_id = 'sa0755' GROUP BY ALL ORDER BY ALL

  SELECT cell, metric, a_median, b_median, effect, lo, hi, mark
  FROM effects('s8ed1e', 'p4tf', 's8ed1e', 'vouch') WHERE mark <> ' ' ORDER BY cell_ord, ord

`index` makes the scoring objects from its checkout's suite.py and
scoring.sql, again whenever either changes (catalog_info's
scoring_version); suite.py loads its own into each session, TEMP.

The physical tables are in schema `raw`, a source's rows each under its
`src`; the views above read them through `jobs`.
"""

import argparse
import collections
import contextlib
import datetime
import functools
import hashlib
import importlib.util
import inspect
import json
import math
import multiprocessing
import os
import queue
import re
import shutil
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]

# A change to the catalog's layout: the next index makes it again, whole.
CATALOG_VERSION = 1
# The extensions' builds for simdb.DUCKDB_VERSION, as DuckDB's repository
# installs them (~/.duckdb/extensions/).
EXTENSIONS = {"ducklake": "ac7595b0", "sqlite_scanner": "5274128"}
# How long S3 keeps a flush (farm.py's S3_KEPT_DAYS).
S3_DAYS = 7
# The env's knobs promoted to jobs' columns: the sims' setting of each
# (SIM_<NAME> in ppspp, STRIMS_SIM_<NAME> in strims-sim).
PROMOTED = {"known_uplink": "KNOWN_UPLINK", "arrivals": "ARRIVALS", "depart": "DEPART", "exp": "EXP"}
# A job's own keys in its env, left out of env_hash: its seed and output.
JOB_KEYS = ("SIM_SEED", "STRIMS_SIM_SEED", "SIM_OUT", "STRIMS_SIM_OUT")
SIMS = {"ppspp": "ppspp", "strims-ppspp:sim": "ppspp", "strims-sim": "strims-sim",
        "strims-sim:scenarios": "strims-sim"}
# The sims' tables that are a job's ids and times, not readings: kept as
# they are where -1 elsewhere is no reading.
ID_COLS = {"k", "src", "sec", "bin", "t_ms", "t_us", "node", "viewer", "seq", "stream"}
# Written tables whose small files the index merges.
WRITTEN = ("rows", "verdicts", "verdict_seconds", "episodes", "derived_events", "farm_index", "fields")
NAME = re.compile(r"^[a-z][a-z0-9_]*$")

_simdb = None


def simdb():
    """hack/sim-db/simdb.py, whose cache, DuckDB and verdicts the catalog
    reads."""
    global _simdb
    if _simdb is None:
        spec = importlib.util.spec_from_file_location("simdb", Path(__file__).with_name("simdb.py"))
        _simdb = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_simdb)
    return _simdb


def q(s):
    return simdb().q(s)


def ident(s):
    return simdb().ident(s)


def lake_root():
    return simdb().cache_root() / "lake"


def catalog_path():
    return lake_root() / "catalog.sqlite"


def data_path():
    return lake_root() / "data"


def attach(write=False):
    """SQL that attaches the catalog as `lake` and uses it: read-only, or
    for the indexer, writing new data under lake/data/ (none of it inlined
    in the SQLite file)."""
    how = (f"(DATA_PATH {q(str(data_path()) + '/')}, DATA_INLINING_ROW_LIMIT 0)" if write else "(READ_ONLY)")
    return ("LOAD ducklake; LOAD sqlite;\n"
            f"ATTACH {q('ducklake:sqlite:' + str(catalog_path()))} AS lake {how};\nUSE lake;\n")


def lake(sql, write=False, out="json", threads=None, memory=None):
    """Runs `sql` over the catalog (simdb.duck): read-only unless `write`."""
    return simdb().duck(attach(write) + sql, out=out, threads=threads, memory=memory)


def extensions():
    """Installs the extensions where missing and checks their builds."""
    names = ", ".join(q(n) for n in EXTENSIONS)
    rows = simdb().duck("INSTALL ducklake; INSTALL sqlite; LOAD ducklake; LOAD sqlite; "
                        f"SELECT extension_name AS n, extension_version AS v FROM duckdb_extensions() "
                        f"WHERE extension_name IN ({names});")
    got = {r["n"]: r["v"] for r in rows}
    bad = {n: got.get(n) for n, v in EXTENSIONS.items() if got.get(n) != v}
    if bad:
        sys.exit(f"DuckDB extensions {bad}, not the pinned {EXTENSIONS}: reinstall them "
                 "(FORCE INSTALL), or pin the new builds in catalog.EXTENSIONS")


# --- scanning -----------------------------------------------------------------


def iso(ts):
    return datetime.datetime.fromtimestamp(ts, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S+00:00")


def scan_cache():
    """The cache's sources: {src: {"files": {table: stat}, "manifest": {...}}},
    a source a dir holding runs.parquet, its tables its Parquet files."""
    root = simdb().cache_root() / "cache"
    out = {}
    if not root.is_dir():
        return out
    for d, dirs, names in os.walk(root):
        dirs[:] = sorted(x for x in dirs if ".tmp" not in x)
        if "runs.parquet" not in names:
            continue
        src = str(Path(d).relative_to(root))
        files = {}
        for n in names:
            t = n[:-len(".parquet")] if n.endswith(".parquet") else None
            if not t or not NAME.match(t):
                continue
            st = os.stat(Path(d) / n)
            files[t] = {"path": str(Path(d) / n), "bytes": st.st_size, "inode": st.st_ino, "mtime_ns": st.st_mtime_ns}
        out[src] = {"files": files, "manifest": simdb().manifest_of(src)}
    return out


def inputs():
    """The metadata files the catalog reads, {path: (size, mtime_ns, kind)}."""
    main = simdb().main_checkout() / "target"
    farm = main / "sim-farm"
    runs = main / "sim-suite" / "farm-runs"
    out = {}

    def add(p, kind):
        with contextlib.suppress(OSError):
            st = os.stat(p)
            out[str(p)] = (st.st_size, st.st_mtime_ns, kind)

    for p in sorted([*runs.glob("*.json"), *(runs / "pending").glob("*.json")]):
        add(p, "suite_index")
    if farm.is_dir():
        with os.scandir(farm) as it:
            for e in it:
                if e.is_dir():
                    add(Path(e.path) / "index.jsonl", "farm_index")
                    add(Path(e.path) / "matrices", "matrices")
    for n, kind in (("costs.jsonl", "costs"), ("s3.jsonl", "s3"), ("bin-hashes.json", "bin_hashes")):
        add(farm / n, kind)
    for p in (simdb().main_checkout() / "hack" / "sim-farm" / "farm.py", ROOT / "hack" / "sim-suite" / "suite.py",
              ROOT / "hack" / "capture" / "analyze.py", Path(__file__), *DOC_FILES):
        add(p, "code")
    return out


# --- metadata files -------------------------------------------------------------


def load_index(path):
    """A farm matrix's index lines as suite.py's load_rows reads them (the
    last line of an id; a torn line skipped), less their rows but the
    trace and reads digest."""
    by = {}
    with contextlib.suppress(OSError):
        for n, line in enumerate(Path(path).read_text().splitlines()):
            try:
                r = json.loads(line)
            except ValueError:
                continue
            if not isinstance(r, dict) or "id" not in r:
                continue
            row = r.get("row") if isinstance(r.get("row"), dict) else {}
            env = r.get("env") if isinstance(r.get("env"), dict) else {}
            by[r["id"]] = {
                "matrix": Path(path).parent.name, "path": str(path), "line": n, "id": r["id"], "k": r.get("key"),
                "arm": r.get("arm"), "commit": r.get("commit"), "binary_sha256": r.get("binary"),
                "test": r.get("test"), "env": {str(a): str(b) for a, b in env.items()},
                "verify_copy": r.get("copy") or None, "host": r.get("host"), "src": r.get("src"),
                "reused_from": r.get("from"), "wall_s": r.get("wall_s"), "rss_mb": r.get("max_rss_mb"),
                "ran_at": r.get("at"), "trace": None if row.get("trace") is None else str(row["trace"]),
                "reads_hash": row.get("reads_hash"), "samples": r.get("reads")}
    return list(by.values())


FARM_INDEX_COLS = {
    "matrix": "VARCHAR", "path": "VARCHAR", "line": "BIGINT", "id": "VARCHAR", "k": "VARCHAR", "arm": "VARCHAR",
    "commit": "VARCHAR", "binary_sha256": "VARCHAR", "test": "VARCHAR", "env": "MAP(VARCHAR, VARCHAR)",
    "verify_copy": "VARCHAR", "host": "VARCHAR", "src": "VARCHAR", "reused_from": "VARCHAR", "wall_s": "DOUBLE",
    "rss_mb": "DOUBLE", "ran_at": "TIMESTAMPTZ", "trace": "VARCHAR", "reads_hash": "VARCHAR", "samples": "BOOLEAN"}
SUITE_RUN_COLS = {
    "exp_id": "VARCHAR", "pending": "BOOLEAN", "knob": "MAP(VARCHAR, VARCHAR)", "sets": "VARCHAR[]",
    "arms": "MAP(VARCHAR, VARCHAR)", "seeds": "BIGINT[]", "base": "VARCHAR", "question": "VARCHAR",
    "written_at": "TIMESTAMPTZ"}
SUITE_CELL_COLS = {
    "exp_id": "VARCHAR", "i": "BIGINT", "cell": "VARCHAR", "sim": "VARCHAR", "test": "VARCHAR", "why": "VARCHAR",
    "env": "MAP(VARCHAR, VARCHAR)", "mem_gib": "DOUBLE"}
MATRIX_COLS = {"farm_run": "VARCHAR", "matrix": "VARCHAR"}
COST_COLS = {
    "run": "VARCHAR", "kind": "VARCHAR", "at": "VARCHAR", "regions": "VARCHAR[]", "interrupted": "BOOLEAN",
    "jobs": "BIGINT", "wall_s": "DOUBLE", "instances": "BIGINT", "spot_interrupted": "BIGINT", "cost": "DOUBLE",
    "boot_cost": "DOUBLE", "busy_cost": "DOUBLE", "idle_cost": "DOUBLE", "busy_slot_h": "DOUBLE",
    "estimate": "JSON", "s3": "JSON"}
S3_COLS = {
    "run": "VARCHAR", "at": "VARCHAR", "source": "VARCHAR", "host": "VARCHAR", "region": "VARCHAR",
    "tier1": "MAP(VARCHAR, BIGINT)", "tier2": "MAP(VARCHAR, BIGINT)", "up": "MAP(VARCHAR, BIGINT)",
    "down": "MAP(VARCHAR, BIGINT)", "stored": "BIGINT"}


@contextlib.contextmanager
def scratch():
    """A scratch dir beside the lake (the same file system), removed after."""
    d = lake_root() / "tmp" / f"{os.getpid()}-{time.monotonic_ns()}"
    d.mkdir(parents=True)
    try:
        yield d
    finally:
        shutil.rmtree(d, ignore_errors=True)


def jsonl(path, recs):
    with open(path, "w") as f:
        for r in recs:
            f.write(json.dumps(r) + "\n")
    return path


def read_jsonl(path, cols):
    return (f"read_json({q(path)}, format='newline_delimited', columns={simdb().struct(cols)}, "
            "ignore_errors=true)")


def tier_cells():
    """suite.py's TIERS as they are now, a record a tier's cell."""
    try:
        suite = suite_module()
    except Exception:
        return []
    return [{"tier": t, "cell": c["name"], "sim": c["sim"], "env": {str(a): str(b) for a, b in c["env"].items()}}
            for t, cells in suite.TIERS.items() for c in cells]


TIER_COLS = {"tier": "VARCHAR", "cell": "VARCHAR", "sim": "VARCHAR", "env": "MAP(VARCHAR, VARCHAR)"}


_suite = None


def suite_module():
    global _suite
    if _suite is None:
        spec = importlib.util.spec_from_file_location("suite", ROOT / "hack" / "sim-suite" / "suite.py")
        _suite = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_suite)
        _suite._simdb = simdb()
    return _suite


def suite_runs(paths):
    """The suite farm runs' indexes: (runs, cells) records."""
    runs, cells = [], []
    by = {}
    for p in sorted(paths, key=lambda p: "/pending/" in p):
        by.setdefault(Path(p).stem, p)
    for exp, p in sorted(by.items()):
        try:
            j = json.loads(Path(p).read_text())
        except (OSError, ValueError):
            continue
        cs = [c for c in j.get("cells") or [] if isinstance(c, dict) and c.get("name")]
        runs.append({"exp_id": exp, "pending": "/pending/" in p, "knob": j.get("knob"),
                     "sets": j.get("values"), "arms": j.get("arms"), "seeds": j.get("seeds"),
                     "base": j.get("base"), "question": j.get("question"),
                     "written_at": iso(os.stat(p).st_mtime)})
        for i, c in enumerate(cs):
            cells.append({"exp_id": exp, "i": i, "cell": c["name"], "sim": c.get("sim"), "test": c.get("test"),
                          "why": c.get("why"), "env": {str(a): str(b) for a, b in (c.get("env") or {}).items()},
                          "mem_gib": c.get("mem_gib")})
    return runs, cells


def matrices(paths):
    out = []
    for p in paths:
        with contextlib.suppress(OSError):
            out += [{"farm_run": Path(p).parent.name, "matrix": m} for m in Path(p).read_text().split()]
    return out


def s3_prices():
    """farm.py's S3 prices: {name: value} of its S3_* and GB constants."""
    out = {}
    try:
        src = (simdb().main_checkout() / "hack" / "sim-farm" / "farm.py").read_text()
    except OSError:
        return out
    for name in ("S3_TIER1", "S3_TIER2", "S3_STORAGE_GB_MONTH", "S3_KEPT_DAYS", "S3_INTER_REGION_GB",
                 "S3_INTERNET_GB", "GB"):
        m = re.search(rf"^{name} = (.+)$", src, re.M)
        if m and re.fullmatch(r"[0-9.e+\-*/ <()]+", m.group(1).strip()):
            out[name] = float(eval(m.group(1), {"__builtins__": {}}))
    return out


# --- docs -----------------------------------------------------------------------

DOC_FILES = (ROOT / "crates" / "strims-sim" / "src" / "tables.rs", ROOT / "crates" / "strims-ppspp" / "tests" / "sim.rs",
             ROOT / "crates" / "strims-sim" / "src" / "measures.rs")
BACKTICK = re.compile(r"`([a-z][a-z0-9_*]*)`")


def doc_tables():
    """The sims' doc tables of their tables ({sim: {table: (grain, text)}}):
    strims-sim's tables module docs, the ppspp sim's write_tables docs."""
    out = {}
    for sim, path, prefix in (("strims-sim", DOC_FILES[0], "//!"), ("ppspp", DOC_FILES[1], "///")):
        with contextlib.suppress(OSError):
            for line in path.read_text().splitlines():
                s = line.strip()
                if not s.startswith(prefix):
                    continue
                s = s[len(prefix):].strip()
                m = re.match(r"^\| `([a-z_]+)` \| (.*?) \| (.*) \|$", s)
                if m:
                    out.setdefault(sim, {})[m.group(1)] = (clean(m.group(2)), clean(m.group(3)))
    return out


def clean(s):
    """Rustdoc text as plain text: links' targets dropped."""
    s = re.sub(r"\[`([^`]+)`\]\([^)]*\)", r"`\1`", s)
    return re.sub(r"\[`([^`]+)`\]", r"`\1`", s)


def paren(text, start):
    """The parenthesized text at `start` (a "(" there), else None."""
    if start >= len(text) or text[start] != "(":
        return None
    depth = 0
    for i in range(start, len(text)):
        depth += {"(": 1, ")": -1}.get(text[i], 0)
        if depth == 0:
            return text[start + 1:i]
    return None


def column_docs(text):
    """A doc table's columns cell, by column: the parenthesized text after
    each backticked name, where one follows it, or the cell's rest after
    one followed by a colon."""
    out = {}
    for m in BACKTICK.finditer(text):
        p = paren(text, m.end() + 1) if text[m.end():m.end() + 1] == " " else None
        if p is None and text[m.end():m.end() + 2] == ": ":
            p = text[m.end() + 2:].strip()
        if p and m.group(1) not in out:
            out[m.group(1)] = p
    return out


def event_docs(text):
    """The events row's text by name: the item of its list that names it
    (between separators, `, `, `; ` or ` and `, at its depth of
    parentheses), after the text the list's parenthesis follows when it's
    in one ("every fault of the scenario: `crash`")."""
    depth, opens, at_depth = 0, [], []
    for ch in text:
        at_depth.append(depth)
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth = max(0, depth - 1)
    seps = re.compile(r", |; | and ")
    ends = lambda lo: text[max(0, lo - 2):lo] in (", ", "; ") or text[max(0, lo - 5):lo] == " and "

    def item(pos):
        """The item around `pos`, and where its parenthesis opens (or -1)."""
        d = at_depth[pos]
        lo = pos
        while lo > 0 and not (at_depth[lo - 1] == d and ends(lo)) and not (text[lo - 1] == "(" and d > 0
                                                                          and at_depth[lo - 1] == d - 1):
            lo -= 1
        hi = pos
        while hi < len(text) and not (at_depth[hi] == d and seps.match(text, hi)) \
                and not (text[hi] == ")" and at_depth[hi] == d):
            hi += 1
        opened = next((i for i in range(lo - 1, -1, -1) if text[i] == "(" and at_depth[i] == d - 1), -1) if d else -1
        return lo, hi, opened

    out = {}
    for m in BACKTICK.finditer(text):
        name = m.group(1)
        if name in out:
            continue
        lo, hi, opened = item(m.start())
        s = text[lo:hi].strip(" ,;")
        if opened > 0:
            clo, _, _ = item(opened - 1)
            context = text[clo:opened].strip(" ,;")
            s = f"{context}: {s}" if context else s
        # the row's last column names the list that follows it
        out[name] = re.sub(r"^`detail`: ", "", s)
    return out


ROW_METHODS = re.compile(r'\.(?:int|num|text|bool|nums|opt_num|opt_int|uint|f64|str|list|ints|field|set)\(\s*"([a-z0-9_]+)"')


def doc_sentences():
    """The sentences of the sims' rustdoc (their tables and rows' modules:
    DOC_FILES) and of suite.py's module docs, each with the backticked
    names it holds, a name's upper-case part (`stalls_per_min_D`) standing
    for any digits."""
    paras = []
    for path in DOC_FILES:
        with contextlib.suppress(OSError):
            cur = []
            for line in path.read_text().splitlines():
                s = line.strip()
                # A doc table's rows are column_docs'.
                if s.startswith(("///", "//!")) and not s[3:].strip().startswith("|"):
                    cur.append(s[3:].strip())
                elif cur:
                    paras.append(" ".join(cur))
                    cur = []
    with contextlib.suppress(OSError):
        doc = suite_module().__doc__ or ""
        paras += [" ".join(p.split()) for p in doc.split("\n\n")]
    out = []
    for p in paras:
        for s in re.split(r"(?<=[.;:])\s+(?=[A-Z(`])", clean(p)):
            names = [n for n in re.findall(r"`([a-z][A-Za-z0-9_{}]*)`", s)]
            pats = [re.compile("^" + re.sub(r"(_[A-Z]+|\{[a-z]+\})", r"_?[0-9a-z_]*", re.escape(n).replace(r"\{", "{")
                                                                   .replace(r"\}", "}")) + "$") for n in names]
            if pats:
                out.append((s, pats))
    return out


def row_docs(names):
    """The rows' fields' docs, {field: text}: the first sentence of the
    sims' and suite.py's docs naming the field (doc_sentences), else the
    comment lines nearest above the line that writes it into a sim's row
    (strims-sim's measures, the ppspp sim's job_row)."""
    sentences = doc_sentences()
    out = {}
    for n in names:
        for s, pats in sentences:
            if any(p.match(n) for p in pats):
                out[n] = s if len(s) <= 600 else s[:597] + "..."
                break
    for path in DOC_FILES[1:]:
        with contextlib.suppress(OSError):
            lines = path.read_text().splitlines()
            for i, line in enumerate(lines):
                for m in ROW_METHODS.finditer(line):
                    name = m.group(1)
                    if name in out:
                        continue
                    j, got = i - 1, []
                    while j >= 0 and i - j <= 12:
                        s = lines[j].strip()
                        if s.startswith("//"):
                            got.insert(0, s.lstrip("/!").strip())
                        elif got:
                            break
                        elif s.endswith(";") or s.endswith("{") or s.endswith("}"):
                            break
                        j -= 1
                    if got:
                        out[name] = clean(" ".join(got))
    return out


def run_metric_docs():
    """analyze.run_metrics' docstring, by metric."""
    doc = simdb().analyze().run_metrics.__doc__ or ""
    out, cur = {}, None
    for line in doc.splitlines():
        m = re.match(r"^\s{4,8}([a-z_]+)\s{2,}(.*)$", line)
        if m:
            cur = m.group(1)
            out[cur] = m.group(2).strip()
        elif cur and line.strip() and re.match(r"^\s{20,}", line):
            out[cur] += " " + line.strip()
        else:
            cur = None
    return out


def suite_metrics():
    """suite.py's METRICS: (name, fields read, better, kind, tie, its SQL,
    the comment above it)."""
    suite = suite_module()
    src = (ROOT / "hack" / "sim-suite" / "suite.py").read_text().splitlines()
    out = []
    for name, sql, better, kind, tie in suite.METRICS:
        line = next((i for i, s in enumerate(src) if re.match(rf'^\s*\("{re.escape(name)}",', s)), None)
        comment = []
        k = (line or 0) - 1
        while line is not None and k >= 0 and src[k].strip().startswith("#"):
            comment.insert(0, src[k].strip().lstrip("# "))
            k -= 1
        fields = sorted({f for _, f in FIELD.findall(sql)})
        out.append((name, fields, better, kind, tie, sql, " ".join(comment) or None))
    return out


# How suite.py's arm_score breaks an arm on a metric (its GUARDRAILS).
GUARD_TEXT = {"lo": "breaks arm_score: ratio CI low past GUARD_RATIO in a cell the base never collapsed",
              "worse": "breaks arm_score: marked worse in a cell",
              "hi": "breaks arm_score: ratio CI high under 1 in a cell",
              "collapsed": "breaks arm_score: more collapsed runs with Fisher p < 0.05 in a cell"}
# A metric's SQL that reads one field, or the first of several it has.
PLAIN = re.compile(r"^(?:coalesce\()?[rvj]\.\w+(?:, [rvj]\.\w+)*\)?$")


def guards():
    """GUARD_TEXT by metric, from suite.py's GUARDRAILS."""
    return {**{m: GUARD_TEXT[g] for m, g in suite_module().GUARDRAILS.items()}, "collapsed": GUARD_TEXT["collapsed"]}


UNITS = ((r"_ms(_|$)", "ms"), (r"_us$", "us"), (r"(_s|_secs)(_|$)", "s"), (r"_per_min$|/min$", "per minute"),
         (r"bytes|_bps$|_wire$|^up_|^down_", "bytes"), (r"share|_rate$|^delivery", "fraction"),
         (r"_mb$", "MB"), (r"_gib$", "GiB"), (r"/useful$", "per useful byte"), (r"_mbit", "Mbit/s"))


def unit_of(name):
    for pat, unit in UNITS:
        if re.search(pat, name):
            return unit
    return None


# --- the index ------------------------------------------------------------------

BASE_SQL = """
CREATE SCHEMA IF NOT EXISTS raw;
CREATE TABLE IF NOT EXISTS raw.sources (src VARCHAR, origin VARCHAR, run VARCHAR, flush VARCHAR, seq BIGINT,
    manifest_at VARCHAR, keys BIGINT, listed MAP(VARCHAR, BIGINT), n_tables BIGINT, verdicts VARCHAR,
    indexed_at TIMESTAMPTZ);
CREATE TABLE IF NOT EXISTS raw.files (src VARCHAR, tbl VARCHAR, path VARCHAR, cache_path VARCHAR, bytes BIGINT,
    inode BIGINT, mtime_ns BIGINT, how VARCHAR);
CREATE TABLE IF NOT EXISTS raw.fields (src VARCHAR, tbl VARCHAR, name VARCHAR, type VARCHAR, sims VARCHAR[]);
CREATE TABLE IF NOT EXISTS raw.inputs (path VARCHAR, size BIGINT, mtime_ns BIGINT, kind VARCHAR);
CREATE TABLE IF NOT EXISTS raw.info (key VARCHAR, value VARCHAR);
"""


def state():
    """The catalog's record of what it holds: (info, files, inputs, sources),
    or None without a catalog, or one of another CATALOG_VERSION."""
    if not catalog_path().exists():
        return None
    try:
        info = {r["key"]: r["value"] for r in lake("SELECT key, value FROM raw.info;")}
    except simdb().DuckError:
        return None
    if info.get("catalog_version") != str(CATALOG_VERSION):
        return None
    files = {(r["src"], r["tbl"]): r for r in lake(
        "SELECT src, tbl, path, cache_path, bytes, inode, mtime_ns, how FROM raw.files;")}
    ins = {r["path"]: (r["size"], r["mtime_ns"], r["kind"]) for r in lake("SELECT * FROM raw.inputs;")}
    srcs = {r["src"]: r for r in lake("SELECT src, verdicts FROM raw.sources;")}
    return info, files, ins, srcs


def wipe():
    """Removes the catalog, for a CATALOG_VERSION it wasn't made under."""
    for p in (catalog_path(), catalog_path().with_name(catalog_path().name + "-wal"),
              catalog_path().with_name(catalog_path().name + "-shm")):
        p.unlink(missing_ok=True)
    shutil.rmtree(data_path(), ignore_errors=True)


def derive_one(src):
    """simdb.derive of one source: None, or why it failed."""
    try:
        simdb().derive([src])
    except Exception as e:
        return f"{src}: {e}"
    return None


def derive_all(srcs, workers, say):
    """simdb.derive over `srcs`, `workers` sources at a time; returns the
    sources it failed (their verdicts wait for the next index)."""
    failed = set()
    if not srcs:
        return failed
    workers = max(1, min(workers, len(srcs)))
    if workers == 1:
        for n, s in enumerate(srcs, 1):
            if err := derive_one(s):
                failed.add(s)
                say(f"verdicts: {err}")
        return failed
    # Forked workers, a share of the sources each: what they run isn't
    # pickled, the loaded modules carried over.
    ctx = multiprocessing.get_context("fork")
    done = ctx.Queue()

    def work(part):
        for s in part:
            done.put((s, derive_one(s)))

    procs = [ctx.Process(target=work, args=(srcs[i::workers],)) for i in range(workers)]
    for p in procs:
        p.start()
    seen = set()
    try:
        while len(seen) < len(srcs):
            try:
                src, err = done.get(timeout=5)
            except queue.Empty:
                if any(p.is_alive() for p in procs):
                    continue
                break
            seen.add(src)
            if err:
                failed.add(src)
                say(f"verdicts: {err}")
            if len(seen) % 50 == 0 or len(seen) == len(srcs):
                say(f"verdicts: {len(seen)}/{len(srcs)} sources")
    finally:
        for p in procs:
            p.join()
    # A worker that died took its sources' verdicts with it.
    return failed | (set(srcs) - seen)


def schemas(paths):
    """The Parquet files' columns, {path: {name: DuckDB type}}."""
    out = collections.defaultdict(dict)
    paths = list(paths)
    for i in range(0, len(paths), 500):
        part = paths[i:i + 500]
        for r in simdb().duck(f"SELECT file_name AS f, name, duckdb_type AS t FROM parquet_schema("
                              f"{simdb().plist(part)}) WHERE duckdb_type IS NOT NULL ORDER BY column_id;"):
            out[r["f"]][r["name"]] = r["t"]
    return out


def lake_columns():
    """The raw tables' columns, {table: {name: type}}."""
    out = collections.defaultdict(dict)
    for r in lake("SELECT table_name AS t, column_name AS c, data_type AS ty FROM information_schema.columns "
                  "WHERE table_catalog = 'lake' AND table_schema = 'raw' ORDER BY ordinal_position;"):
        out[r["t"]][r["c"]] = r["ty"]
    return out


def link(cache_file, tbl, src, inode):
    """A hard link of a cache file in the lake's data dir (a copy where the
    file systems differ)."""
    d = data_path() / "linked" / tbl
    d.mkdir(parents=True, exist_ok=True)
    dest = d / f"{src.replace('/', '~')}.{inode}.parquet"
    if not dest.exists():
        try:
            os.link(cache_file, dest)
        except OSError:
            shutil.copyfile(cache_file, dest)
    return dest


def json_type(t):
    """A row field's DuckDB type from json_group_structure's, or None while
    only nulls were seen."""
    if isinstance(t, list):
        inner = t[0] if t else "NULL"
        if isinstance(inner, str) and inner in ("UBIGINT", "BIGINT", "HUGEINT", "UHUGEINT", "DOUBLE", "NULL"):
            return "DOUBLE[]"
        return "VARCHAR"
    if isinstance(t, dict):
        return "VARCHAR"
    if t in ("UBIGINT", "BIGINT", "HUGEINT", "UHUGEINT", "DOUBLE"):
        return "DOUBLE"
    if t == "BOOLEAN":
        return "BOOLEAN"
    if t == "NULL":
        return None
    return "VARCHAR"


def index(workers=None, quiet=False, wait=True):
    """Brings the catalog up to date (the module docs); returns what it did."""
    say = (lambda *a: None) if quiet else (lambda *a: print(*a, file=sys.stderr))
    lake_root().mkdir(parents=True, exist_ok=True)
    with simdb().locked(lake_root() / "index.lock", block=wait) as mine:
        if not mine:
            say("another index holds lake/index.lock")
            return None
        extensions()
        t0 = time.monotonic()
        st = state()
        if st is None:
            if catalog_path().exists():
                say(f"the catalog isn't CATALOG_VERSION {CATALOG_VERSION}'s: making it again")
            wipe()
            data_path().mkdir(parents=True, exist_ok=True)
            # A cache source's runs and scalars hold simdb's columns, and an
            # older one may lack some.
            own = [f"CREATE TABLE raw.{t} ({', '.join(f'{ident(n)} {ty}' for n, ty in c.items())});"
                   for t, c in (("runs", simdb().RUN_COLS), ("scalars", simdb().SCALAR_COLS))]
            lake(BASE_SQL + "\n".join(own), write=True, out=None)
            st = ({}, {}, {}, {})
        info, have, seen, done_srcs = st
        cache = scan_cache()
        ins = inputs()
        version = simdb().derived_version()
        now = {(s, t): f for s, c in cache.items() for t, f in c["files"].items()}
        same = lambda a, b: (a["inode"], a["bytes"], a["mtime_ns"]) == (b["inode"], b["bytes"], b["mtime_ns"])
        gone = [key for key in have if key not in now]
        changed = [key for key, f in now.items() if key in have and not same(have[key], f)]
        new = [key for key in now if key not in have]
        ins_changed = {p for p, v in ins.items() if seen.get(p) != v} | {p for p in seen if p not in ins}
        need_verdicts = sorted(s for s in cache if done_srcs.get(s, {}).get("verdicts") != version)
        runs_redo = sorted({s for s, t in [*changed, *new] if t == "runs"})
        if not (gone or changed or new or ins_changed or need_verdicts) and info.get("verdict_version") == version:
            cols = lake_columns()
            scoring, sv = lake_scoring(cols)
            if not sv or info.get("scoring_version") == sv:
                say(f"the catalog is up to date ({time.monotonic() - t0:.1f} s)")
                return {"changed": False}
            # Only the scoring objects changed: they, their docs and their
            # version, alone.
            with scratch() as tmp:
                lake("\n".join(["BEGIN;", *scoring, "DELETE FROM raw.info WHERE key = 'scoring_version';",
                                 f"INSERT INTO raw.info VALUES ('scoring_version', {q(sv)});",
                                 *catalogs_sql(cols, tmp), "COMMIT;"]), write=True, out=None)
            say(f"index: the scoring macros made again ({time.monotonic() - t0:.1f} s)")
            return {"changed": True, "scoring": True}
        say(f"index: {len(new)} new files, {len(changed)} changed, {len(gone)} gone; "
            f"{len(ins_changed)} metadata files changed; verdicts for {len(need_verdicts)} sources")
        failed = derive_all(need_verdicts, workers or max(1, min(8, (os.cpu_count() or 2) // 2)), say)
        with scratch() as tmp:
            kind_of = {p: v[2] for p, v in seen.items()} | {p: v[2] for p, v in ins.items()}
            sql = build(cache, have, gone, changed, new, runs_redo, ins, ins_changed, kind_of,
                        [s for s in need_verdicts if s not in failed], version, tmp, failed)
            say(f"index: writing ({time.monotonic() - t0:.0f} s in)")
            try:
                lake(sql, write=True, out=None)
                null_stats()
            except simdb().DuckError as e:
                # The transaction rolled back: the catalog is as it was. The
                # SQL and the files it read stay in lake/failed/.
                failed_dir = lake_root() / "failed"
                shutil.rmtree(failed_dir, ignore_errors=True)
                shutil.copytree(tmp, failed_dir)
                (failed_dir / "index.sql").write_text(sql.replace(str(tmp), str(failed_dir)))
                sys.exit(f"index: {e}\n(the SQL: {failed_dir / 'index.sql'}; the catalog is as it was)")
        say(f"index: compacting ({time.monotonic() - t0:.0f} s in)")
        compact()
        say(f"index: done in {time.monotonic() - t0:.0f} s")
        return {"changed": True, "new": len(new), "gone": len(gone), "changed_files": len(changed),
                "verdicts": len(need_verdicts)}


def build(cache, have, gone, changed, new, runs_redo, ins, ins_changed, kind_of, need_verdicts, version, tmp,
          failed=()):
    """The index's one transaction, as SQL."""
    cols = lake_columns()
    pre, sql = [], ["BEGIN;"]
    drop = sorted({*gone, *changed})
    # A source's tables leave first: its files gone or changed.
    by_tbl = collections.defaultdict(list)
    for s, t in drop:
        by_tbl[t].append(s)
    for t, srcs in sorted(by_tbl.items()):
        lst = ", ".join(q(s) for s in srcs)
        if t in cols:
            sql.append(f"DELETE FROM raw.{ident(t)} WHERE src IN ({lst});")
        # The rows are the runs', and so are their fields.
        if t == "runs" and "rows" in cols:
            sql.append(f"DELETE FROM raw.rows WHERE src IN ({lst});")
        sql.append(f"DELETE FROM raw.fields WHERE tbl = {q('rows' if t == 'runs' else t)} AND src IN ({lst});")
    if drop:
        sql.append("DELETE FROM raw.files WHERE (src, tbl) IN (" + ", ".join(f"({q(s)}, {q(t)})" for s, t in drop)
                   + ");")
    # The sim tables' files, linked where their types are the lake's.
    add = sorted({*changed, *new})
    types = schemas([cache[s]["files"][t]["path"] for s, t in add])
    files_rows, fields_rows = [], []
    sims = sims_of(cache, add)
    # A column new to the lake takes the type most of the new files give
    # it; the files giving another are cast.
    votes = collections.defaultdict(collections.Counter)
    for s, t in add:
        for n, ty in types.get(cache[s]["files"][t]["path"], {}).items():
            if n not in cols.get(t, {}):
                votes[(t, n)][ty] += 1
    for s, t in add:
        f = cache[s]["files"][t]
        have_cols = types.get(f["path"], {})
        if not have_cols:
            continue
        table = cols.setdefault(t, {})
        new_cols = {n: votes[(t, n)].most_common(1)[0][0] for n in have_cols if n not in table}
        if not table:
            first = {"k": "VARCHAR", "src": "VARCHAR", **new_cols}
            sql.append(f"CREATE TABLE raw.{ident(t)} ({', '.join(f'{ident(n)} {ty}' for n, ty in first.items())});")
            table.update(first)
        for n, ty in new_cols.items():
            if n not in table:
                sql.append(f"ALTER TABLE raw.{ident(t)} ADD COLUMN {ident(n)} {ty};")
                table[n] = ty
        if all(table[n] == ty for n, ty in have_cols.items()):
            dest = link(f["path"], t, s, f["inode"])
            how = "linked"
        else:
            # Written beside the links, cast, before the transaction, and
            # registered as the links are.
            dest = data_path() / "converted" / t / f"{s.replace('/', '~')}.{f['inode']}.parquet"
            dest.parent.mkdir(parents=True, exist_ok=True)
            casts = ", ".join(f"TRY_CAST({ident(n)} AS {table[n]}) AS {ident(n)}" for n in have_cols)
            pre.append(f"COPY (SELECT {casts} FROM read_parquet({q(f['path'])})) TO {q(str(dest))} "
                       "(FORMAT parquet, COMPRESSION zstd);")
            how = "converted"
        sql.append(f"CALL ducklake_add_data_files('lake', {q(t)}, {q(str(dest))}, schema => 'raw', "
                   "allow_missing => true);")
        files_rows.append({"src": s, "tbl": t, "path": str(dest), "cache_path": f["path"],
                           "bytes": f["bytes"], "inode": f["inode"], "mtime_ns": f["mtime_ns"], "how": how})
        if t != "runs":
            fields_rows += [{"src": s, "tbl": t, "name": n, "type": ty, "sims": sims.get(s, [])}
                            for n, ty in have_cols.items() if n not in ("k", "src")]
    if files_rows:
        sql.append(f"INSERT INTO raw.files BY NAME SELECT * FROM {read_jsonl(jsonl(tmp / 'files.jsonl', files_rows), FILE_COLS)};")
    if fields_rows:
        sql.append(f"INSERT INTO raw.fields BY NAME SELECT * FROM "
                   f"{read_jsonl(jsonl(tmp / 'fields.jsonl', fields_rows), FIELD_COLS)};")
    sql += rows_sql(runs_redo, cache, cols)
    sql += verdicts_sql(need_verdicts, cache, cols, version)
    meta, parsed = metadata_sql(ins, ins_changed, kind_of, cols, tmp)
    sql += meta
    sql += sources_sql(cache, failed, version, tmp)
    for t in ("verdicts", "verdict_seconds", "episodes", "derived_events"):
        if t in cols:
            sql.append(f"DELETE FROM raw.{t} WHERE src NOT IN (SELECT src FROM raw.sources);")
    sql.append("DELETE FROM raw.inputs;")
    sql.append("INSERT INTO raw.inputs BY NAME SELECT * FROM " + read_jsonl(
        jsonl(tmp / "inputs.jsonl", [{"path": p, "size": a, "mtime_ns": b, "kind": c} for p, (a, b, c) in ins.items()]),
        {"path": "VARCHAR", "size": "BIGINT", "mtime_ns": "BIGINT", "kind": "VARCHAR"}) + ";")
    sql += main_sql(cols, version, tmp, ins, parsed)
    sql.append("COMMIT;")
    return "\n".join([*pre, *sql])


def null_stats():
    """Marks every column of the lake's tables as holding NULLs in its
    table-wide statistics, in the SQLite catalog itself (DuckLake, attached,
    keeps it locked). The pinned DuckLake build doesn't mark a column a
    registered file lacks (allow_missing) as holding NULLs, and the
    optimizer folds `IS NULL` and coalesce() on the column by those
    statistics: the file's rows read NULL there, yet `x IS NULL` is false.
    The table-wide flag only loses that folding; the files' own statistics,
    which prune them, are right. A reader between the index's commit and
    this sees the folding for that moment."""
    simdb().duck(f"LOAD sqlite; ATTACH {q(str(catalog_path()))} AS lake_meta (TYPE sqlite);\n"
                 "UPDATE lake_meta.ducklake_table_column_stats SET contains_null = 1 WHERE contains_null = 0;",
                 out=None)


FILE_COLS = {"src": "VARCHAR", "tbl": "VARCHAR", "path": "VARCHAR", "cache_path": "VARCHAR", "bytes": "BIGINT",
             "inode": "BIGINT", "mtime_ns": "BIGINT", "how": "VARCHAR"}
FIELD_COLS = {"src": "VARCHAR", "tbl": "VARCHAR", "name": "VARCHAR", "type": "VARCHAR", "sims": "VARCHAR[]"}


def sims_of(cache, keys):
    """The sims (normalized) of each source among `keys`' runs."""
    srcs = sorted({s for s, _ in keys if "runs" in cache.get(s, {}).get("files", {})})
    out = {}
    for i in range(0, len(srcs), 500):
        part = [cache[s]["files"]["runs"]["path"] for s in srcs[i:i + 500]]
        for r in simdb().duck(f"SELECT src, list(DISTINCT sim ORDER BY sim) AS sims FROM read_parquet("
                              f"{simdb().plist(part)}, union_by_name=true) GROUP BY src;"):
            out[r["src"]] = sorted({SIMS.get(x, x) for x in r["sims"] if x})
    return out


def rows_sql(srcs, cache, cols):
    """raw.rows from the sources' runs: a column a row field, of the type its
    values have (numbers DOUBLE, lists of them DOUBLE[], strings VARCHAR),
    the columns growing with the fields later rows hold; and raw.fields'
    record of each source's fields."""
    srcs = [s for s in srcs if "runs" in cache.get(s, {}).get("files", {})]
    if not srcs:
        return []
    paths = [cache[s]["files"]["runs"]["path"] for s in srcs]
    struct = {}
    for i in range(0, len(paths), 200):
        got = simdb().duck(f"SELECT json_group_structure(row_json::JSON) AS s FROM read_parquet("
                           f"{simdb().plist(paths[i:i + 200])}, union_by_name=true) WHERE row_json IS NOT NULL;")
        s = got[0]["s"] if got else None
        if isinstance(s, str):
            try:
                s = json.loads(s)
            except ValueError:
                s = None
        for n, t in (s if isinstance(s, dict) else {}).items():
            ty = json_type(t)
            if ty and n not in ("k", "src") and NAME.match(n):
                struct.setdefault(n, ty)
    out = []
    table = cols.setdefault("rows", {})
    if not table:
        out.append("CREATE TABLE raw.rows (k VARCHAR, src VARCHAR);")
        table.update({"k": "VARCHAR", "src": "VARCHAR"})
    for n, ty in sorted(struct.items()):
        if n not in table:
            out.append(f"ALTER TABLE raw.rows ADD COLUMN {ident(n)} {ty};")
            table[n] = ty
    fields = {n: ty for n, ty in table.items() if n not in ("k", "src")}
    if fields:
        shape = q(json.dumps({n: ty for n, ty in fields.items()}))
        lst = ", ".join(q(s) for s in srcs)
        # A flush holding one job under two cells (a matrix that reused a
        # row of its own run's) holds its row twice: one is the job's.
        out.append(f"INSERT INTO raw.rows BY NAME SELECT k, src, unnest(json_transform(row_json, {shape})) "
                   f"FROM raw.runs WHERE src IN ({lst}) QUALIFY row_number() OVER (PARTITION BY k, src ORDER BY cell) = 1;")
        sims = f"CASE {' '.join(f'WHEN sim = {q(a)} THEN {q(b)}' for a, b in SIMS.items())} ELSE sim END"
        out.append(f"INSERT INTO raw.fields BY NAME SELECT src, 'rows' AS tbl, name, any_value(ty) AS type, "
                   f"list(DISTINCT {sims} ORDER BY {sims}) AS sims FROM (SELECT src, sim, unnest(json_keys(row_json)) "
                   f"AS name, NULL::VARCHAR AS ty FROM raw.runs WHERE src IN ({lst})) GROUP BY ALL;")
        out.append(f"UPDATE raw.fields f SET type = c.data_type FROM information_schema.columns c "
                   f"WHERE f.tbl = 'rows' AND f.type IS NULL AND c.table_catalog = 'lake' AND c.table_schema = 'raw' "
                   f"AND c.table_name = 'rows' AND c.column_name = f.name;")
    return out


def verdicts_sql(srcs, cache, cols, version):
    """The verdict tables of `srcs` under `version`, from simdb.derive's
    files; versions but the current and the one before leave."""
    sm = simdb()
    out = []
    tables = {**{t: dict(c) for t, c in sm.DERIVED.items()}, "verdicts": dict(sm.VERDICT_COLS)}
    for t, base in tables.items():
        files = [str(sm.derived_dir(t) / (s.replace("/", "~") + ".parquet")) for s in srcs]
        files = [f for f in files if Path(f).exists()]
        have_cols = dict(base)
        if files:
            for f, c in schemas(files).items():
                for n, ty in c.items():
                    if n != "v":
                        have_cols.setdefault(n, ty)
        table = cols.setdefault(t, {})
        if not table:
            first = {**have_cols, "verdict_version": "VARCHAR"}
            out.append(f"CREATE TABLE raw.{t} ({', '.join(f'{ident(n)} {ty}' for n, ty in first.items())});")
            table.update(first)
        for n, ty in have_cols.items():
            if n not in table:
                out.append(f"ALTER TABLE raw.{t} ADD COLUMN {ident(n)} {ty};")
                table[n] = ty
        if srcs:
            out.append(f"DELETE FROM raw.{t} WHERE verdict_version = {q(version)} AND src IN "
                       f"({', '.join(q(s) for s in srcs)});")
        if files:
            out.append(f"INSERT INTO raw.{t} BY NAME SELECT COLUMNS(lambda c: c <> 'v'), {q(version)} AS verdict_version "
                       f"FROM read_parquet({sm.plist(files)}, union_by_name=true, hive_partitioning=false);")
        out.append(f"DELETE FROM raw.{t} WHERE verdict_version NOT IN (SELECT {q(version)} UNION ALL "
                   f"SELECT value FROM raw.info WHERE key = 'verdict_version');")
    return out


def metadata_sql(ins, changed, kind_of, cols, tmp):
    """The farm's and the suite's metadata, from the files that `changed`
    (`kind_of` their kinds, theirs before too): the SQL, and the farm index
    lines it parsed."""
    out, parsed = [], []
    kinds = collections.defaultdict(list)
    for p, (_, _, kind) in ins.items():
        kinds[kind].append(p)
    moved = {kind_of.get(p) for p in changed}

    def table(name, spec):
        if name not in cols:
            out.append(f"CREATE TABLE raw.{name} ({', '.join(f'{ident(n)} {ty}' for n, ty in spec.items())});")
            cols[name] = dict(spec)

    table("farm_index", FARM_INDEX_COLS)
    redo = sorted(p for p in changed if kind_of.get(p) == "farm_index")
    if redo:
        out.append(f"DELETE FROM raw.farm_index WHERE path IN ({', '.join(q(p) for p in redo)});")
        parsed = [r for p in redo if p in ins for r in load_index(p)]
        if parsed:
            out.append(f"INSERT INTO raw.farm_index BY NAME SELECT * FROM "
                       f"{read_jsonl(jsonl(tmp / 'farm_index.jsonl', parsed), FARM_INDEX_COLS)};")
    table("tier_cells", TIER_COLS)
    out.append("DELETE FROM raw.tier_cells;")
    if tiers := tier_cells():
        out.append(f"INSERT INTO raw.tier_cells BY NAME SELECT * FROM "
                   f"{read_jsonl(jsonl(tmp / 'tier_cells.jsonl', tiers), TIER_COLS)};")
    whole = {"suite_index": ("suite_runs", "suite_cells"), "matrices": ("matrices",), "costs": ("costs",),
             "s3": ("s3_ledger",), "bin_hashes": ("bin_hashes",)}
    specs = {"suite_runs": SUITE_RUN_COLS, "suite_cells": SUITE_CELL_COLS, "matrices": MATRIX_COLS,
             "costs": COST_COLS, "s3_ledger": S3_COLS, "bin_hashes": {"name": "VARCHAR", "sha256": "VARCHAR"}}
    for kind, names in whole.items():
        for n in names:
            table(n, specs[n])
        if kind not in moved:
            continue
        for n in names:
            out.append(f"DELETE FROM raw.{n};")
        if kind == "suite_index":
            runs, cells = suite_runs(kinds[kind])
            for n, got in (("suite_runs", runs), ("suite_cells", cells)):
                if got:
                    out.append(f"INSERT INTO raw.{n} BY NAME SELECT * FROM "
                               f"{read_jsonl(jsonl(tmp / f'{n}.jsonl', got), specs[n])};")
        elif kind == "matrices":
            got = matrices(kinds[kind])
            if got:
                out.append(f"INSERT INTO raw.matrices BY NAME SELECT * FROM "
                           f"{read_jsonl(jsonl(tmp / 'matrices.jsonl', got), MATRIX_COLS)};")
        elif kind in ("costs", "s3"):
            for p in kinds[kind]:
                out.append(f"INSERT INTO raw.{names[0]} BY NAME SELECT * FROM {read_jsonl(p, specs[names[0]])};")
        elif kind == "bin_hashes":
            for p in kinds[kind]:
                with contextlib.suppress(OSError, ValueError):
                    j = json.loads(Path(p).read_text())
                    got = [{"name": n, "sha256": v.get("sha256")} for n, v in j.items() if isinstance(v, dict)]
                    if got:
                        out.append(f"INSERT INTO raw.bin_hashes BY NAME SELECT * FROM "
                                   f"{read_jsonl(jsonl(tmp / 'bin_hashes.jsonl', got), specs['bin_hashes'])};")
    return out, parsed


def sources_sql(cache, failed, version, tmp):
    """raw.sources, a cache source each: its run, flush, manifest, the sim
    tables it holds and the verdict_version its verdicts are of (none where
    simdb.derive `failed`)."""
    recs = []
    tables = set(simdb().TABLES)
    for s, c in cache.items():
        parts = s.split("/")
        m = c["manifest"]
        last = parts[-1]
        recs.append({
            "src": s, "origin": parts[0], "run": parts[1] if len(parts) > 1 else None,
            "flush": "/".join(parts[2:]) or None,
            "seq": int(last) if parts[0] == "farm" and last.isdigit() else -1,
            "manifest_at": m.get("at"), "keys": len(m.get("keys") or []),
            "listed": {t: (v or {}).get("bytes") for t, v in (m.get("tables") or {}).items()},
            "n_tables": sum(1 for t in c["files"] if t in tables),
            "verdicts": None if s in failed else version, "indexed_at": iso(time.time())})
    cols = {"src": "VARCHAR", "origin": "VARCHAR", "run": "VARCHAR", "flush": "VARCHAR", "seq": "BIGINT",
            "manifest_at": "VARCHAR", "keys": "BIGINT", "listed": "MAP(VARCHAR, BIGINT)", "n_tables": "BIGINT",
            "verdicts": "VARCHAR", "indexed_at": "TIMESTAMPTZ"}
    return ["DELETE FROM raw.sources;",
            f"INSERT INTO raw.sources BY NAME SELECT * FROM {read_jsonl(jsonl(tmp / 'sources.jsonl', recs), cols)};"]


# --- the main tables and views --------------------------------------------------


def sim_case(col):
    return f"CASE {' '.join(f'WHEN {col} = {q(a)} THEN {q(b)}' for a, b in SIMS.items())} ELSE {col} END"


def knob(env, name):
    return f"coalesce({env}['STRIMS_SIM_{name}'], {env}['SIM_{name}'])"


JOBS_SQL = """
CREATE OR REPLACE TABLE main.jobs AS
WITH cand AS (
    SELECT r.k, r.src, r.sim, r.test, r.cell, r.arm, r.seed, r.env, r.trace, r.frames, r.bytes, r.wall_s, r.rss_mb,
           r."at", r.bin_ms, json_extract_string(r.row_json, '$.reads_hash') AS reads_hash,
           row_number() OVER (PARTITION BY r.k ORDER BY s.n_tables DESC, coalesce(s.manifest_at, '') DESC,
                              s.seq DESC, r.src DESC) AS rn
    FROM raw.runs r JOIN raw.sources s USING (src)),
canon AS (SELECT * EXCLUDE (rn) FROM cand WHERE rn = 1),
fi AS (SELECT k, arg_min(f, (f.reused_from IS NOT NULL AND f.reused_from <> f.matrix, f.ran_at, f.line)) AS f
       FROM raw.farm_index f WHERE k IS NOT NULL GROUP BY k),
held AS (SELECT src, list(tbl ORDER BY tbl) AS tables FROM raw.files GROUP BY src),
j AS (
    SELECT k, c.src AS csrc, c.sim AS csim, c.test AS ctest, c.seed AS cseed, c.cell, c.arm, c.trace AS ctrace,
           c.frames, c.bytes, c.wall_s AS cwall, c.rss_mb AS crss, c."at" AS cat, c.bin_ms, c.reads_hash AS creads,
           fi.f, coalesce(fi.f.env, json_transform(c.env, '"MAP(VARCHAR, VARCHAR)"')) AS env
    FROM (SELECT k FROM canon UNION SELECT k FROM fi) keys LEFT JOIN canon c USING (k) LEFT JOIN fi USING (k)),
s AS (
    SELECT *, coalesce({csim},
                       CASE WHEN len(list_filter(map_keys(env), lambda x: x LIKE 'STRIMS_SIM_%')) > 0 THEN 'strims-sim'
                            WHEN len(list_filter(map_keys(env), lambda x: x LIKE 'SIM_%')) > 0 THEN 'ppspp' END) AS nsim
    FROM j)
SELECT k, nsim AS sim, coalesce(ctest, f.test) AS test,
       coalesce(cseed, TRY_CAST(env[CASE WHEN nsim = 'ppspp' THEN 'SIM_SEED' ELSE 'STRIMS_SIM_SEED' END] AS BIGINT)) AS seed,
       f.commit AS commit, f.binary_sha256 AS binary_sha256, env,
       CASE WHEN env['STRIMS_SIM_RTC'] IS NOT NULL THEN 'webrtc' WHEN env['STRIMS_SIM_TCP'] = '1' THEN 'tcp' END
           AS transport,
       env['STRIMS_SIM_RTC'] AS rtc_browsers,
       {promoted},
       md5(array_to_string(list_sort(list_transform(
           list_filter(map_entries(env), lambda e: NOT list_contains({job_keys}, e.key)),
           lambda e: e.key || '=' || e.value)), chr(10)))[1:16] AS env_hash,
       coalesce(ctrace, f.trace) AS trace, coalesce(creads, f.reads_hash) AS reads_hash,
       coalesce(f.wall_s, cwall) AS wall_s, coalesce(f.rss_mb, crss) AS rss_mb,
       coalesce(TRY_CAST(cat AS TIMESTAMPTZ), f.ran_at) AS ran_at,
       coalesce(csrc, f.src) AS src, split_part(coalesce(csrc, f.src), '/', 1) AS origin,
       split_part(coalesce(csrc, f.src), '/', 2) AS run, csrc IS NOT NULL AS cached,
       coalesce(h.tables, []) AS tables, cell AS stored_cell, coalesce(arm, f.arm) AS stored_arm, bin_ms, frames,
       bytes
FROM s LEFT JOIN held h ON h.src = s.csrc;
"""

EXPERIMENTS_SQL = """
CREATE OR REPLACE TABLE main.experiment_cells AS
WITH fenv AS (
    SELECT matrix, arg_min(env, (reused_from IS NOT NULL AND reused_from <> matrix, line)) AS env
    FROM raw.farm_index GROUP BY matrix)
SELECT c.exp_id, c.cell, {csim} AS sim, c.test, c.why,
       CASE WHEN cardinality(c.env) > 0 THEN c.env ELSE map_from_entries(list_filter(map_entries(f.env),
           lambda e: NOT list_contains({job_keys} || coalesce(map_values(r.knob), []), e.key))) END AS env,
       c.mem_gib, f.matrix
FROM raw.suite_cells c JOIN raw.suite_runs r USING (exp_id)
LEFT JOIN fenv f ON f.matrix = c.exp_id || '-' || lpad(c.i::VARCHAR, 2, '0')
UNION ALL BY NAME
SELECT src AS exp_id, cell, any_value({rsim}) AS sim, any_value(test) AS test,
       any_value(json_transform(env, '"MAP(VARCHAR, VARCHAR)"')) AS env
FROM raw.runs WHERE src LIKE 'suite/%' GROUP BY src, cell;

CREATE OR REPLACE TABLE main.experiment_jobs AS
SELECT u.* EXCLUDE (line), j.src FROM (
    SELECT c.exp_id, c.cell,
           CASE WHEN r.knob[c.sim] IS NULL THEN f.arm ELSE coalesce(f.env[r.knob[c.sim]], f.arm) END AS arm_label,
           f.arm AS ref, f.env[r.knob[c.sim]] AS knob_value, f.k,
           TRY_CAST(f.env[CASE WHEN c.sim = 'ppspp' THEN 'SIM_SEED' ELSE 'STRIMS_SIM_SEED' END] AS BIGINT) AS seed,
           f.matrix, f.reused_from IS NOT NULL AND f.reused_from <> f.matrix AS reused, f.src AS flush, f.line
    FROM raw.suite_cells c JOIN raw.suite_runs r USING (exp_id)
    JOIN raw.farm_index f ON f.matrix = c.exp_id || '-' || lpad(c.i::VARCHAR, 2, '0')
    WHERE f.src IS NOT NULL AND f.k IS NOT NULL
    UNION ALL BY NAME
    SELECT src AS exp_id, cell, arm AS arm_label, arm AS ref, k, seed, false AS reused, src AS flush, 0 AS line
    FROM raw.runs WHERE src LIKE 'suite/%') u
LEFT JOIN main.jobs j USING (k)
QUALIFY row_number() OVER (PARTITION BY u.exp_id, u.cell, u.arm_label, u.seed, u.k ORDER BY u.reused, u.line DESC) = 1;

CREATE OR REPLACE TABLE main.experiments AS
WITH ej AS (
    SELECT e.exp_id, count(*) AS jobs, count(DISTINCT e.cell) AS cells, list(DISTINCT e.seed ORDER BY e.seed) AS seeds_seen,
           list(DISTINCT e.arm_label ORDER BY e.arm_label) AS arm_labels,
           min(j.ran_at) FILTER (WHERE NOT e.reused) AS first_ran, max(j.ran_at) FILTER (WHERE NOT e.reused) AS last_ran
    FROM main.experiment_jobs e LEFT JOIN main.jobs j USING (k) GROUP BY e.exp_id),
own AS (SELECT matrix, count(*) AS n FROM raw.farm_index
        WHERE reused_from IS NULL OR reused_from = matrix GROUP BY matrix),
runs AS (SELECT m.farm_run, m.matrix, coalesce(o.n, 0) AS n, sum(coalesce(o.n, 0)) OVER (PARTITION BY m.farm_run) AS total
         FROM raw.matrices m LEFT JOIN own o USING (matrix)),
cost AS (SELECT run AS farm_run, sum(cost) AS cost FROM raw.costs GROUP BY ALL),
s3 AS (SELECT farm_run, sum(cost_usd) AS s3 FROM main.s3 GROUP BY ALL),
share AS (
    SELECT c.exp_id, r.farm_run, sum(r.n) / nullif(any_value(r.total), 0) AS share
    FROM main.experiment_cells c JOIN runs r USING (matrix) GROUP BY ALL),
spend AS (
    SELECT exp_id, list(DISTINCT farm_run ORDER BY farm_run) AS farm_runs, sum(share * cost.cost) AS cost_usd,
           sum(share * s3.s3) AS s3_usd
    FROM share LEFT JOIN cost USING (farm_run) LEFT JOIN s3 USING (farm_run) GROUP BY exp_id),
tiers AS (
    SELECT exp_id, list(tier ORDER BY tier) AS tiers FROM (
        SELECT e.exp_id, t.tier, count(DISTINCT t.cell) AS n, any_value(t.size) AS size
        FROM main.experiment_cells e
        JOIN (SELECT *, count(*) OVER (PARTITION BY tier) AS size FROM raw.tier_cells) t
          ON t.cell = e.cell AND list_sort(map_entries(t.env)) = list_sort(map_entries(e.env))
        GROUP BY ALL)
    WHERE n = size GROUP BY exp_id),
refs AS (
    SELECT e.exp_id, map_from_entries(list(DISTINCT {{'key': f.arm, 'value': f.commit}})) AS arms
    FROM main.experiment_cells e JOIN raw.farm_index f USING (matrix)
    WHERE f.arm IS NOT NULL AND f.commit IS NOT NULL AND (f.reused_from IS NULL OR f.reused_from = f.matrix)
    GROUP BY e.exp_id)
SELECT r.exp_id, 'farm' AS kind, coalesce(ej.first_ran, r.written_at) AS started_at, ej.last_ran AS finished_at,
       coalesce(t.tiers, []) AS tiers, coalesce(r.seeds, ej.seeds_seen) AS seeds, r.knob, r.sets,
       coalesce(r.arms, refs.arms) AS arms, ej.arm_labels, coalesce(r.base, r.sets[1]) AS base, r.question,
       r.pending, coalesce(ej.cells, 0) AS cells, coalesce(ej.jobs, 0) AS jobs, sp.farm_runs, sp.cost_usd, sp.s3_usd
FROM raw.suite_runs r LEFT JOIN ej USING (exp_id) LEFT JOIN spend sp USING (exp_id) LEFT JOIN tiers t USING (exp_id)
LEFT JOIN refs USING (exp_id)
UNION ALL BY NAME
SELECT src AS exp_id, 'local' AS kind, min(TRY_CAST("at" AS TIMESTAMPTZ)) AS started_at,
       max(TRY_CAST("at" AS TIMESTAMPTZ)) AS finished_at, list(DISTINCT seed ORDER BY seed) AS seeds,
       list(DISTINCT arm ORDER BY arm) AS arm_labels, false AS pending, count(DISTINCT cell) AS cells,
       count(*) AS jobs
FROM raw.runs WHERE src LIKE 'suite/%' GROUP BY src;
"""

COSTS_SQL = """
CREATE OR REPLACE VIEW main.costs AS
SELECT run AS farm_run, kind, TRY_CAST("at" AS TIMESTAMPTZ) AS finished_at, regions, interrupted, jobs, wall_s,
       instances, spot_interrupted, cost AS cost_usd, boot_cost AS boot_usd, busy_cost AS busy_usd,
       idle_cost AS idle_usd, busy_slot_h, TRY_CAST(s3->>'cost' AS DOUBLE) AS s3_usd,
       TRY_CAST(estimate->>'cost' AS DOUBLE) AS estimate_usd
FROM raw.costs;
CREATE OR REPLACE VIEW main.s3 AS
SELECT run AS farm_run, TRY_CAST("at" AS TIMESTAMPTZ) AS logged_at, source, host, region, tier1, tier2, up, down,
       stored,
       coalesce(list_sum(map_values(tier1)), 0) * {S3_TIER1} + coalesce(list_sum(map_values(tier2)), 0) * {S3_TIER2}
       + (coalesce(list_sum(list_transform(list_filter(map_entries(up), lambda e: e.key NOT IN ('here', region)),
                                           lambda e: e.value)), 0)
          + coalesce(list_sum(list_transform(list_filter(map_entries(down),
                                                         lambda e: e.key NOT IN ('internet', region)),
                                             lambda e: e.value)), 0)) / {GB} * {S3_INTER_REGION_GB}
       + coalesce(down['internet'], 0) / {GB} * {S3_INTERNET_GB}
       + coalesce(stored, 0) / {GB} * {S3_STORAGE_GB_MONTH} * {S3_KEPT_DAYS} / 30 AS cost_usd
FROM raw.s3_ledger;
"""

AVAILABILITY_SQL = """
CREATE OR REPLACE TABLE main.availability AS
WITH loc AS (SELECT j.k, f.tbl, 'local' AS state, f.src, f.bytes FROM main.jobs j JOIN raw.files f USING (src)),
holders AS (SELECT DISTINCT k, src FROM raw.runs UNION SELECT DISTINCT k, src FROM raw.farm_index
            WHERE src IS NOT NULL AND k IS NOT NULL),
listed AS (
    SELECT h.k, e.key AS tbl, h.src, e.value AS bytes, TRY_CAST(s.manifest_at AS TIMESTAMPTZ) AS up_at
    FROM holders h JOIN raw.sources s USING (src), unnest(map_entries(s.listed)) AS t(e) WHERE s.origin = 'farm'),
unpulled AS (
    SELECT f.k, x.tbl, f.src, NULL::BIGINT AS bytes, f.ran_at AS up_at
    FROM raw.farm_index f, (VALUES ('sim-data'), ('sim-samples')) x(tbl)
    WHERE f.src IS NOT NULL AND f.k IS NOT NULL AND (x.tbl = 'sim-data' OR f.samples)
      AND f.src NOT IN (SELECT src FROM raw.sources) AND f.k NOT IN (SELECT k FROM raw.runs)),
remote AS (
    SELECT k, tbl, CASE WHEN up_at + INTERVAL {days} DAY > now() THEN 's3' ELSE 'expired' END AS state, src, bytes
    FROM (SELECT * FROM listed UNION ALL SELECT * FROM unpulled))
SELECT * FROM loc
UNION ALL
SELECT * FROM (SELECT * FROM remote ANTI JOIN loc USING (k, tbl))
QUALIFY row_number() OVER (PARTITION BY k, tbl ORDER BY state = 's3' DESC, bytes NULLS LAST, src) = 1;
"""

EXP_RUNS_SQL = """
CREATE OR REPLACE VIEW main.exp_runs AS
SELECT e.*, j.* EXCLUDE (k, seed, src) FROM main.experiment_jobs e JOIN main.jobs j USING (k);
CREATE OR REPLACE VIEW main.catalog_info AS SELECT key, value FROM raw.info;
"""


def canon(t):
    """A raw table's rows of the jobs' own sources (jobs.src). An inner
    join, a job a row of jobs: a filter on k reaches jobs through it, and
    the source it gives prunes the table's files (a dynamic filter on src),
    as a semi join's doesn't."""
    return f"FROM raw.{ident(t)} x JOIN main.jobs j_ ON j_.k = x.k AND j_.src = x.src"


NUMERIC = ("BIGINT", "INTEGER", "DOUBLE", "FLOAT", "HUGEINT", "SMALLINT", "TINYINT", "UBIGINT", "UINTEGER", "DECIMAL")


def view_sql(t, cols, name, viewers):
    """The view `name` of sim table `t` (its columns `cols`), normalized as
    the module docs say; `viewers` the viewers table's columns, for the
    viewers' stays."""
    num = lambda c: cols[c].split("(")[0] in NUMERIC
    rest = [c for c in cols if c not in ("k", "src")]
    nullify = lambda c: f"nullif(x.{ident(c)}, -1) AS {ident(c)}" if num(c) else f"x.{ident(c)}"
    plain = lambda c: f"x.{ident(c)}"
    head = f"CREATE OR REPLACE VIEW main.{ident(name)} AS SELECT"
    if t == "viewers":
        node = ("coalesce(x.node, x.host)" if "node" in cols and "host" in cols else
                "x.node" if "node" in cols else "x.host" if "host" in cols else "NULL::BIGINT")
        sel = ["x.k", "x.src", "x.viewer", f"{node} AS node"]
        special = {"up": "nullif(x.up, -1) AS up", "hostile": "coalesce(x.hostile, false) AS hostile",
                   "churned": "x.churned::BOOLEAN AS churned",
                   "arrive_ms": "CASE WHEN x.arrive_ms >= 0 THEN x.arrive_ms END AS arrive_ms"}
        sel += [special.get(c, plain(c)) for c in rest if c not in ("viewer", "node", "host")]
        return f"{head} {', '.join(sel)} {canon(t)};"
    if t == "viewer_seconds":
        stay = []
        if "arrive_ms" in viewers:
            stay.append("(w.arrive_ms IS NULL OR w.arrive_ms < 0 OR (x.sec + 1) * 1000 > w.arrive_ms)")
        if "depart_ms" in viewers:
            stay.append("(w.depart_ms IS NULL OR x.sec * 1000 < w.depart_ms)")
        sel = ["x.k", "x.src"] + [("nullif(x.lag_ms, -1) AS lag_ms" if c == "lag_ms" else plain(c)) for c in rest]
        join = "LEFT JOIN raw.viewers w ON w.k = x.k AND w.src = x.src AND w.viewer = x.viewer" if stay else ""
        return f"{head} {', '.join(sel)} {canon(t)} {join}" + (f" WHERE {' AND '.join(stay)}" if stay else "") + ";"
    if t == "events":
        sel = ["x.k", "x.src"]
        for c in rest:
            sel.append({"node": "nullif(x.node, -1) AS node", "peer": "nullif(x.peer, -1) AS peer",
                        "value": "CASE WHEN x.kind = 'parent_lost' AND x.value = -1 THEN NULL ELSE x.value END AS value"
                        }.get(c, plain(c)))
            if c == "t_us":
                sel.append("x.t_us / 1000.0 AS t_ms")
        if "detail" in cols:
            sel.append("CASE WHEN contains(x.detail, '=') THEN map_from_entries(list_transform("
                       "string_split(trim(x.detail), ' '), lambda p: {'key': split_part(p, '=', 1), "
                       "'value': p[strpos(p, '=') + 1:]})) END AS detail_map")
        return f"{head} {', '.join(sel)} {canon(t)};"
    if t in ("seconds", "bins"):
        sel = ["x.k", "x.src"] + [plain(c) if c in ID_COLS else nullify(c) for c in rest]
        return f"{head} {', '.join(sel)} {canon(t)};"
    if t == "node_bins":
        sel = ["x.k", "x.src"] + [("nullif(x.viewer, -1) AS viewer" if c == "viewer" else plain(c)) for c in rest]
        return f"{head} {', '.join(sel)} {canon(t)};"
    if t == "forecast":
        sel = ["x.k", "x.src"] + [nullify(c) if c in ("horizon_ms", "held_ms", "mean_ms") else plain(c) for c in rest]
        return f"{head} {', '.join(sel)} {canon(t)};"
    if t == "parents":
        sel = ["x.k", "x.src"] + [nullify(c) if c == "parent" else plain(c) for c in rest]
        return f"{head} {', '.join(sel)} {canon(t)};"
    return f"{head} x.* {canon(t)};"


# The catalog's own names, which a sim table's view doesn't take (it is
# sim_<table> then).
OWN = {"experiments", "experiment_cells", "experiment_jobs", "exp_runs", "jobs", "commits", "costs", "s3",
       "availability", "rows", "scalars", "catalog_info", "metric_catalog", "event_kinds", "table_catalog",
       "verdicts", "verdict_seconds", "episodes", "derived_events", "verdicts_all", "verdict_seconds_all",
       "episodes_all", "derived_events_all"}
# The raw tables that aren't sims' tables.
META = {"sources", "files", "fields", "inputs", "info", "runs", "rows", "scalars", "farm_index", "suite_runs",
        "suite_cells", "matrices", "costs", "s3_ledger", "bin_hashes", "verdicts", "verdict_seconds",
        "episodes", "derived_events", "tier_cells"}


def sim_tables(cols):
    """The raw tables that are sims' tables, {table: its view's name}."""
    return {t: (t if t not in OWN else f"sim_{t}") for t in sorted(cols) if t not in META}


def main_sql(cols, version, tmp, ins, parsed):
    """The main tables, made again, and the views."""
    sm = simdb()
    out = [JOBS_SQL.format(csim=sim_case("csim"), promoted=",\n       ".join(
        f"{knob('env', v)} AS {n}" for n, v in PROMOTED.items()),
        job_keys="[" + ", ".join(q(k) for k in JOB_KEYS) + "]")]
    prices = s3_prices()
    if len(prices) < 7:
        prices = dict.fromkeys(("S3_TIER1", "S3_TIER2", "S3_INTER_REGION_GB", "S3_INTERNET_GB",
                                "S3_STORAGE_GB_MONTH", "S3_KEPT_DAYS"), 0.0) | {"GB": 1.0}
    out.append(COSTS_SQL.format(**{k: repr(v) for k, v in prices.items()}))
    out.append(EXPERIMENTS_SQL.format(csim=sim_case("c.sim"), rsim=sim_case("sim"),
                                      job_keys="[" + ", ".join(q(k) for k in JOB_KEYS) + "]"))
    out.append(AVAILABILITY_SQL.format(days=S3_DAYS))
    out.append(EXP_RUNS_SQL)
    out += commits_sql(tmp, ins, parsed)
    for t, name in sim_tables(cols).items():
        out.append(view_sql(t, cols[t], name, cols.get("viewers", {})))
    for t in ("rows", "scalars"):
        if t in cols:
            out.append(f"CREATE OR REPLACE VIEW main.{t} AS SELECT x.* {canon(t)};")
    for t in ("verdicts", "verdict_seconds", "episodes", "derived_events"):
        if t in cols:
            out.append(f"CREATE OR REPLACE VIEW main.{t}_all AS SELECT x.* {canon(t)};")
            out.append(f"CREATE OR REPLACE VIEW main.{t} AS SELECT * FROM main.{t}_all "
                       f"WHERE verdict_version = (SELECT value FROM raw.info WHERE key = 'verdict_version');")
    info = {"catalog_version": str(CATALOG_VERSION), "verdict_version": version,
            "analyze_verdict_version": sm.analyze().VERDICT_VERSION, "duckdb_version": sm.DUCKDB_VERSION,
            "ducklake_version": EXTENSIONS["ducklake"], "sqlite_version": EXTENSIONS["sqlite_scanner"],
            "indexed_at": iso(time.time())}
    scoring, sv = lake_scoring(cols)
    if sv:
        info["scoring_version"] = sv
    out.append("DELETE FROM raw.info;")
    out.append("INSERT INTO raw.info VALUES " + ", ".join(f"({q(k)}, {q(v)})" for k, v in info.items()) + ";")
    out += scoring
    out += catalogs_sql(cols, tmp)
    return out


def lake_scoring(cols):
    """The catalog's scoring objects (scoring_sql, from this checkout's
    suite.py) over the raw tables' columns `cols`, and their
    scoring_version; none, with a warning, when suite.py doesn't load."""
    try:
        sv = scoring_version()
        return [scoring_sql(scoring_cols({t: cols.get(t, {}) for t in ("rows", "verdicts")}))], sv
    except Exception as e:
        print(f"index: no scoring macros: suite.py's METRICS don't load ({e})", file=sys.stderr)
        return [], None


def commits_sql(tmp, ins, parsed):
    """main.commits: the commits the farm's indexes and the suite's name,
    from git: subject, author, date, whether main holds it, and the local
    branches whose tip it is."""
    sm = simdb()
    shas = set()
    with contextlib.suppress(sm.DuckError):
        shas |= {r["c"] for r in lake("SELECT DISTINCT commit AS c FROM raw.farm_index WHERE commit IS NOT NULL;")}
    for p, (_, _, kind) in ins.items():
        if kind == "suite_index":
            with contextlib.suppress(OSError, ValueError, AttributeError):
                shas |= {v for v in (json.loads(Path(p).read_text()).get("arms") or {}).values() if v}
    shas |= {r["commit"] for r in parsed if r.get("commit")}
    shas = sorted(s for s in shas if isinstance(s, str) and re.fullmatch(r"[0-9a-f]{7,40}", s))
    recs = []
    if shas:
        def run(*args, stdin=None):
            return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, input=stdin).stdout

        known = [line.split()[0] for line in run("cat-file", "--batch-check", stdin="\n".join(shas) + "\n").splitlines()
                 if line.split()[1:2] == ["commit"]]
        main = set(run("rev-list", "main").split())
        tips = collections.defaultdict(list)
        for line in run("for-each-ref", "--format=%(objectname) %(refname:short)", "refs/heads").splitlines():
            sha, _, ref = line.partition(" ")
            tips[sha].append(ref)
        log = run("log", "--no-walk=unsorted", "--format=%H%x1f%cI%x1f%an%x1f%s", *known) if known else ""
        for line in log.splitlines():
            sha, date, author, subject = (line.split("\x1f") + ["", "", ""])[:4]
            recs.append({"sha": sha, "committed_at": date, "author": author, "subject": subject,
                         "on_main": sha in main, "branches": sorted(tips.get(sha, []))})
        got = {r["sha"] for r in recs}
        recs += [{"sha": s, "branches": []} for s in shas if s not in got]
    cols = {"sha": "VARCHAR", "committed_at": "TIMESTAMPTZ", "author": "VARCHAR", "subject": "VARCHAR",
            "on_main": "BOOLEAN", "branches": "VARCHAR[]"}
    out = [f"CREATE OR REPLACE TABLE main.commits ({', '.join(f'{n} {t}' for n, t in cols.items())});"]
    if recs:
        out.append(f"INSERT INTO main.commits BY NAME SELECT * FROM "
                   f"{read_jsonl(jsonl(tmp / 'commits.jsonl', recs), cols)};")
    return out


# --- self-description -----------------------------------------------------------

# The catalog's tables: (kind, grain, keys, joins, description). The sims'
# tables add their own, from the sims' docs (doc_tables).
TABLE_DOCS = {
    "experiments": (
        "table", "an experiment", "exp_id", "experiment_cells, experiment_jobs USING (exp_id)",
        "A suite farm run (kind 'farm'; exp_id its name, as suite.py farm printed it) or a local suite invocation "
        "(kind 'local'; exp_id its cache source, suite/<worktree>/<invocation>, which holds the jobs it ran itself, "
        "not the ones it kept from earlier invocations). started_at and finished_at: its own jobs' first and last "
        "ran_at (a farm run's pending index's time before any ran); tiers: suite.py's TIERS, as they are now, whose "
        "every cell it ran; seeds; knob (the env variable by sim) and sets of a --sets run; arms (name: commit, from "
        "its index) and arm_labels (experiment_jobs' arms); base: the arm the others are scored against; question "
        "(suite.py farm --question); pending: its index still waits for the farm to start it; farm_runs: the farm "
        "runs that ran its matrices; cost_usd and s3_usd: those runs' costs and S3 ledger lines, each shared out "
        "by the jobs its matrices ran of the run's."),
    "experiment_cells": (
        "table", "an experiment's cell", "exp_id, cell", "experiments USING (exp_id); experiment_jobs USING (exp_id, cell)",
        "A cell as the experiment ran it: sim, test, why, env (its settings), mem_gib (the farm slot's), and the "
        "farm matrix that ran it (<exp_id>-<i>). Cell names repeat across tiers (browsers keeps robust's names "
        "with other settings): (exp_id, cell) is a cell."),
    "experiment_jobs": (
        "table", "an experiment's job", "exp_id, cell, arm_label, seed -> k",
        "jobs and every result table USING (k, src); experiment_cells USING (exp_id, cell)",
        "The jobs an experiment scored, by cell, arm and seed. arm_label is the arm as suite.py and simdb.py name "
        "it (simdb.arm_label): a --sets run's set (the value the job's env gives the run's knob), else the matrix's "
        "arm (a --refs name); ref is the matrix's arm, knob_value the env's value of the knob; src the job's source "
        "(jobs.src), which joins the result tables USING (k, src) and prunes their files. A job the farm reused "
        "from another run or matrix (reused, its row from flush) counts in each experiment it was reused for, so "
        "a k can be in several experiments and cells. A farm job whose data left S3 stays listed (availability)."),
    "exp_runs": (
        "view", "an experiment's job", "exp_id, cell, arm_label, seed -> k", "as experiment_jobs",
        "experiment_jobs with its jobs' columns (jobs, but k and seed)."),
    "jobs": (
        "table", "a job", "k", "every table USING (k, src); commits ON commits.sha = jobs.commit",
        "Every job the cache or the farm's indexes know: sim ('ppspp' or 'strims-sim'), test, seed, commit and "
        "binary_sha256 (the farm's), env (its settings, MAP) and knobs promoted from it (transport: 'tcp' or "
        "'webrtc' (rtc_browsers the browsers) or NULL for the sim's own model; known_uplink, arrivals, depart and "
        "exp: the sims' SIM_*/STRIMS_SIM_* settings), env_hash (its settings but seed and output, so equal settings "
        "match across experiments), trace and reads_hash (the digests a rerun must reproduce), wall_s and rss_mb "
        "(its host's), ran_at, and src: the cache source its tables come from (simdb.py's source_ranks: the one "
        "holding the most tables, then the later flush, then the later seq), or the farm flush that holds them "
        "when the cache doesn't (cached false). tables: the tables src holds; stored_cell and stored_arm: the "
        "cell (a farm matrix, or a local cell) and arm its source stored it under; bin_ms: its bins' width."),
    "commits": (
        "table", "a commit", "sha", "jobs ON jobs.commit = commits.sha; experiments' arms values",
        "The commits the farm's jobs and the experiments' arms name, from git: subject, author, committed_at, "
        "on_main (main holds it), branches (the local branches whose tip it is). One git doesn't know has NULLs."),
    "costs": (
        "view", "a farm run's cost line", "farm_run", "experiments' farm_runs",
        "The farm's cost of each run (target/sim-farm/costs.jsonl): cost_usd (boot, busy and idle), its jobs, "
        "wall time, instances, the S3 cost when it ended and the estimate's cost."),
    "s3": (
        "view", "an S3 ledger line", "farm_run", "costs USING (farm_run)",
        "The farm's S3 ledger (target/sim-farm/s3.jsonl): each host's, the farm's and each pull's requests and "
        "bytes (source 'host', 'run', 'pull', ...), priced as farm.py prices them (cost_usd)."),
    "availability": (
        "table", "a job's table", "k, tbl", "jobs USING (k, src)",
        "Where each job's tables are: state 'local' (the catalog holds tbl of the job's source), 's3' (a farm flush "
        "of it lists tbl and S3 still keeps it: fetch copies it in) or 'expired' (S3 kept it 7 days); src the "
        "source, bytes its file's size (the flush's, shared by its jobs). A job no cache source ever held has "
        "'sim-data' (its flush's tables) and, where its samples landed, 'sim-samples' (reads, forecast)."),
    "rows": (
        "view", "a job", "k", "jobs USING (k, src)",
        "Each job's row (the sims' one-line JSON result), a column a field: numbers DOUBLE, lists of them "
        "DOUBLE[], strings VARCHAR. Fields a sim or a commit doesn't write are NULL. metric_catalog describes each."),
    "scalars": (
        "view", "a number of a job's row", "k, name", "jobs USING (k, src)",
        "The rows' numbers in long form, as simdb.py folds them: bools 1.0, list elements name[i], non-finite "
        "numbers and strings left out."),
    "verdicts": (
        "view", "a job", "k", "jobs USING (k, src)",
        "analyze.py's verdict and run metrics of each job (simdb.py's derived `verdicts`), of the current "
        "verdict_version: onset (the first unhealthy trigger's second), trees, delivery, latency (each trigger's "
        "onset), recovered, absorbing, feasible ('True', 'False' or 'step'), unhealthy (the share of unhealthy "
        "seconds), lag_p50, lag_max, collapsed (an unhealthy spell of COLLAPSED_SECS lasting past startup_s), "
        "startup_s, then analyze.run_metrics' columns. verdicts_all holds every version kept."),
    "verdict_seconds": (
        "view", "a job's second", "k, sec", "jobs USING (k, src); seconds USING (k, src, sec)",
        "Each second's verdict: phase (startup, after_down, after_up, steady), healthy and what judged it "
        "(trees_bad, deliv_bad, lag_high, lag_rise), hit_share, delivery and lag_q (the honest viewers' q0.9 lag)."),
    "episodes": (
        "view", "a job's spell", "k, kind, start", "jobs USING (k, src)",
        "A run's healthy, unhealthy and collapse spells: [start, stop) seconds, len."),
    "derived_events": (
        "view", "a job's derived event", "k, sec, kind", "jobs USING (k, src)",
        "analyze.events: start, seed_down, seed_up, present_join, present_leave, onset, recovered, collapse "
        "(value: a step's new uplink over the old, how many viewers came or went, a collapse's length)."),
    "catalog_info": (
        "view", "a setting", "key", "",
        "The catalog's versions (catalog_version, verdict_version: the current verdicts', analyze's "
        "VERDICT_VERSION, the DuckDB and extension builds) and when it was last indexed."),
    "metric_catalog": (
        "table", "a column or metric", "tbl, name", "table_catalog ON table_catalog.name = metric_catalog.tbl",
        "Every column of the result tables and every suite.py metric (tbl 'suite'): type, unit (from its name), "
        "description (the sims' and analyze.py's docs), better ('lower', 'higher' or NULL), kind ('ratio' or 'diff'), "
        "tie (the band suite.py calls a tie), guard (how suite.py's arm_score uses it), the fields a suite metric "
        "reads and its source, the sims that write it, and when it was first and last seen."),
    "event_kinds": (
        "table", "an event kind of a sim", "kind, sim", "events, derived_events ON kind",
        "The kinds of events seen (sim 'derived' for derived_events'): the docs' description, what `value` means, "
        "how many, and when first and last seen."),
    "table_catalog": (
        "table", "a table", "name", "metric_catalog ON metric_catalog.tbl = table_catalog.name",
        "This list: each table's grain, keys, joins and description."),
}
# The sims' tables' normalization in the views.
NORMALIZED = {
    "viewers": "node is the viewer's node in both sims (strims-sim's host); churned BOOLEAN; arrive_ms NULL for a "
               "viewer there from the start; up NULL without a limit; hostile false where a sim didn't say.",
    "viewer_seconds": "only the seconds the viewer was there (its stay overlaps the second), in both sims; lag_ms "
                      "NULL in a second without a read.",
    "events": "t_ms = t_us / 1000; node and peer NULL for none; a parent_lost's value NULL if the viewer never "
              "stopped counting on the parent; detail_map: detail as a MAP where it is key=value pairs.",
    "seconds": "-1 (no reading) NULL.", "bins": "-1 (no reading) NULL.",
    "node_bins": "viewer NULL for the seed and hostile hosts.",
    "stalls": "The unit played is the sim's: a frame in strims-sim, a chunk in the ppspp sim, whose stalls are "
              "one chunk's lateness and so often far shorter than a frame's. A unit read twice counts at its "
              "first read. macros.sql's playback(delay_ms) plays reads the same way at any delay.",
    "parents": "parent NULL for none. Only hybrid push gives a stream push parents: empty in other modes.",
    "forecast": "horizon_ms, held_ms and mean_ms NULL where -1 (no estimate, never held).",
}
STANDARD_KEYS = {"seconds": "k, sec", "viewer_seconds": "k, viewer, sec", "viewers": "k, viewer",
                 "events": "k, t_us, kind, node", "bins": "k, bin", "node_bins": "k, bin, node",
                 "reads": "k, viewer, t_ms", "forecast": "k, node, peer, seq",
                 "stalls": "k, viewer, delay_ms, t_ms", "parents": "k, node, stream, t_ms"}
STANDARD_JOINS = {"seconds": "jobs USING (k, src); verdict_seconds USING (k, src, sec)",
                  "viewer_seconds": "viewers USING (k, src, viewer)", "reads": "viewers USING (k, src, viewer)",
                  "events": "viewers USING (k, src, node)", "node_bins": "viewers USING (k, src, node)",
                  "forecast": "viewers USING (k, src, node)", "stalls": "viewers USING (k, src, viewer)",
                  "parents": "viewers USING (k, src, node)", "viewers": "reads, viewer_seconds, stalls USING (k, src, viewer)"}
VERDICT_DOCS = {
    "onset": "the first second a trigger held (trees, delivery or latency), NULL if none",
    "trees": "the first second of TREES_SECS with the smoothed hit share under TREES_SHARE",
    "delivery": "the first second the delivery share fell under DELIVERY_SHARE",
    "latency": "the first second of LAG_HIGH_SECS of lag past LAG_HIGH_MS or rising past LAG_SLOPE",
    "recovered": "the first second of RECOVER_SECS healthy ones after the onset",
    "absorbing": "it never recovered, or only ABSORB_SECS past the seed's last step up",
    "feasible": "whether the seed's uplink could carry the stream ('True', 'False', 'step')",
    "unhealthy": "the share of unhealthy seconds", "lag_p50": "the median of the lag q0.9 series",
    "lag_max": "the largest lag q0.9", "collapsed": "an unhealthy spell of COLLAPSED_SECS lasting past startup_s",
    "startup_s": "the seconds a collapse must last past: STARTUP_SECS, or 0 when every honest viewer arrived later",
    "phase": "startup, after_down, after_up (within STEP_SECS of a seed step) or steady",
    "healthy": "no trigger held this second", "trees_bad": "the smoothed hit share under TREES_SHARE",
    "deliv_bad": "the delivery share under DELIVERY_SHARE", "lag_high": "the lag q0.9 past LAG_HIGH_MS",
    "lag_rise": "the lag rising past LAG_SLOPE ms/s over LAG_SLOPE_SECS while past LAG_SLOPE_FLOOR_MS",
    "hit_share": "the honest viewers' hits over what they needed",
    "lag_q": "the honest present viewers' nearest-rank q0.9 lag, ms",
    "verdict_version": "the verdicts' version (simdb.derived_version)",
}
DERIVED_EVENT_DOCS = {
    "start": "the stream's start", "seed_down": "the seed's uplink stepped down (value: new over old)",
    "seed_up": "the seed's uplink stepped up (value: new over old)",
    "present_join": "viewers present rose (value: by how many)", "present_leave": "viewers present fell (value: by how many)",
    "onset": "the verdict's onset", "recovered": "the verdict's recovery", "collapse": "a collapse spell began (value: its length, s)",
}


def catalogs_sql(cols, tmp):
    """metric_catalog, event_kinds and table_catalog, made again."""
    docs = doc_tables()
    out = []
    # table_catalog
    tables = []
    for name, (kind, grain, keys, joins, desc) in TABLE_DOCS.items():
        tables.append({"name": name, "kind": kind, "grain": grain, "keys": keys, "joins": joins, "description": desc})
    for t, view in sim_tables(cols).items():
        grain = "; ".join(f"{s}: {docs[s][t][0]}" for s in sorted(docs) if t in docs[s]) or f"a row of {t}"
        desc = " ".join(f"{s}: {docs[s][t][1]}" for s in sorted(docs) if t in docs[s]) or f"A sim's {t} table."
        if t in NORMALIZED:
            desc += f" Normalized: {NORMALIZED[t]}"
        tables.append({"name": view, "kind": "view", "grain": grain, "keys": STANDARD_KEYS.get(t, "k"),
                       "joins": STANDARD_JOINS.get(t, "jobs USING (k)"), "description": desc})
    tables += scoring_docs()
    tcols = {"name": "VARCHAR", "kind": "VARCHAR", "grain": "VARCHAR", "keys": "VARCHAR", "joins": "VARCHAR",
             "description": "VARCHAR"}
    out.append(f"CREATE OR REPLACE TABLE main.table_catalog AS SELECT * FROM "
               f"{read_jsonl(jsonl(tmp / 'table_catalog.jsonl', tables), tcols)};")
    # metric_catalog
    col_docs = collections.defaultdict(dict)
    for sim, ts in docs.items():
        for t, (_, text) in ts.items():
            for c, d in column_docs(text).items():
                col_docs[(t, c)][sim] = d
    names = {c for c in (n for t in cols.values() for n in t) if c not in ("k", "src")} | {m[0] for m in suite_metrics()}
    rdocs, mdocs = row_docs(names), run_metric_docs()
    metrics = suite_metrics()
    plain, guard = {}, guards()
    for name, fields, better, kind, tie, text, _ in metrics:
        if PLAIN.match(text):
            for f in fields:
                plain.setdefault(f, (better, kind, tie, name))
    recs = []
    names = {"rows": "rows", **sim_tables(cols)}
    for t, view in [*names.items(), *((v, v) for v in ("verdicts", "verdict_seconds", "episodes", "derived_events"))]:
        for c in cols.get(t, {}):
            if c in ("k", "src"):
                continue
            by = col_docs.get((t, c), {})
            desc = (rdocs.get(c) if t == "rows" else
                    (VERDICT_DOCS.get(c) or mdocs.get(c) or mdocs.get(c.replace("_censored", "")))
                    if t in ("verdicts", "verdict_seconds", "episodes", "derived_events") else
                    ("; ".join(f"{s}: {d}" for s, d in sorted(by.items())) if len(set(by.values())) > 1
                     else next(iter(by.values()), None))) or rdocs.get(c)
            better, kind, tie, metric = plain.get(c, (None, None, None, None))
            recs.append({"name": c, "tbl": view, "type": cols[t][c], "unit": unit_of(c), "description": desc,
                         "better": better, "kind": kind, "tie": tie, "guard": guard.get(metric or c),
                         "fields": None, "source": None, "metric": metric})
    for name, fields, better, kind, tie, text, comment in metrics:
        recs.append({"name": name, "tbl": "suite", "type": "DOUBLE", "unit": unit_of(name),
                     "description": comment or rdocs.get(name) or next((rdocs[f] for f in fields if f in rdocs), None),
                     "better": better, "kind": kind, "tie": tie, "guard": guard.get(name),
                     "fields": fields, "source": text, "metric": name})
    recs.append({"name": "collapsed", "tbl": "suite", "type": "BOOLEAN", "unit": None,
                 "description": "collapsed runs of a cell, A against B, judged by their counts' Fisher exact p "
                                "(suite.py collapses and fisher)", "better": "lower", "kind": "count",
                 "tie": None, "guard": guard["collapsed"], "fields": ["collapsed"], "source": None,
                 "metric": "collapsed"})
    mcols = {"name": "VARCHAR", "tbl": "VARCHAR", "type": "VARCHAR", "unit": "VARCHAR", "description": "VARCHAR",
             "better": "VARCHAR", "kind": "VARCHAR", "tie": "DOUBLE", "guard": "VARCHAR", "fields": "VARCHAR[]",
             "source": "VARCHAR", "metric": "VARCHAR"}
    view_of = " ".join(f"WHEN f.tbl = {q(t)} THEN {q(v)}" for t, v in names.items())
    out.append(f"""CREATE OR REPLACE TABLE main.metric_catalog AS
WITH seen AS (
    SELECT CASE {view_of} ELSE f.tbl END AS tbl, f.name, list_distinct(flatten(list(f.sims))) AS sims,
           min(TRY_CAST(s.manifest_at AS TIMESTAMPTZ)) AS first_seen,
           max(TRY_CAST(s.manifest_at AS TIMESTAMPTZ)) AS last_seen
    FROM raw.fields f LEFT JOIN raw.sources s USING (src) GROUP BY ALL)
SELECT d.name, d.tbl, d.type, d.unit, d.description, d.better, d.kind, d.tie, d.guard, d.fields, d.source,
       d.metric, list_sort(s.sims) AS sims, s.first_seen, s.last_seen
FROM {read_jsonl(jsonl(tmp / 'metric_catalog.jsonl', recs), mcols)} d LEFT JOIN seen s USING (tbl, name)
ORDER BY d.tbl, d.name;""")
    # event_kinds
    ev = []
    for sim, ts in docs.items():
        if "events" in ts:
            for kind, text in event_docs(ts["events"][1]).items():
                m = re.search(r"`value` ([^,;)]+)", text)
                ev.append({"kind": kind, "sim": sim, "description": text, "value_meaning": m.group(1).strip() if m else None})
    for kind, text in DERIVED_EVENT_DOCS.items():
        m = re.search(r"\(value: ([^)]+)\)", text)
        ev.append({"kind": kind, "sim": "derived", "description": text, "value_meaning": m.group(1) if m else None})
    ecols = {"kind": "VARCHAR", "sim": "VARCHAR", "description": "VARCHAR", "value_meaning": "VARCHAR"}
    seen = []
    if "events" in cols:
        seen.append("SELECT j.sim, e.kind, count(*) AS n, min(j.ran_at) AS first_seen, max(j.ran_at) AS last_seen "
                    "FROM raw.events e JOIN main.jobs j USING (k, src) GROUP BY ALL")
    if "derived_events" in cols:
        seen.append("SELECT 'derived' AS sim, e.kind, count(*) AS n, min(j.ran_at), max(j.ran_at) "
                    "FROM main.derived_events e JOIN main.jobs j USING (k) GROUP BY ALL")
    seen_sql = " UNION ALL ".join(seen) or ("SELECT NULL::VARCHAR AS sim, NULL::VARCHAR AS kind, 0 AS n, "
                                            "NULL::TIMESTAMPTZ AS first_seen, NULL::TIMESTAMPTZ AS last_seen WHERE false")
    out.append(f"""CREATE OR REPLACE TABLE main.event_kinds AS
WITH seen AS ({seen_sql}),
doc AS (SELECT * FROM {read_jsonl(jsonl(tmp / 'event_kinds.jsonl', ev), ecols)})
SELECT s.kind, s.sim, d.description, d.value_meaning, s.n, s.first_seen, s.last_seen
FROM seen s LEFT JOIN doc d USING (kind, sim) ORDER BY s.sim, s.kind;""")
    return out


# --- scoring ----------------------------------------------------------------------

SCORING = Path(__file__).with_name("scoring.sql")
# A metric's SQL reads a job's row (r), verdict (v) and job (j).
FIELD = re.compile(r"\b([rvj])\.([a-z_][a-z0-9_]*)\b")
CONST = re.compile(r"@\{([a-z_][a-z0-9_]*)\}")
ROW_FIELD = re.compile(r"@\{r\.([a-z0-9_]+)\}")
MACRO = re.compile(r"^CREATE OR REPLACE (MACRO|TABLE|VIEW) ([a-z_0-9]+)\s*(\([^)]*\))?", re.M)
TEMP = re.compile(r"^CREATE OR REPLACE (MACRO|TABLE|VIEW) ", re.M)


def scoring_consts(suite=None):
    """scoring.sql's @{name} constants: suite.py's and analyze.py's, and
    t975_at's expansion (t975_expansion)."""
    s, a = suite or suite_module(), simdb().analyze()
    z, g = t975_expansion()
    return {"guard_ratio": s.GUARD_RATIO, "lag_ceiling": a.DELIVERY_TARGET_MS,
            "delay_target_ms": a.DELIVERY_TARGET_MS, "delay_max_ms": a.DELIVERY_MAX_MS,
            "join_grace_secs": a.JOIN_GRACE_SECS, "t975_df": T975_DF, "t975_z": double(z),
            **{f"t975_g{i}": double(c) for i, c in enumerate(g, 1)}}


def double(x):
    """`x` as a DuckDB DOUBLE literal, the float itself (a decimal literal
    is a DECIMAL, cast to DOUBLE through its unscaled integer)."""
    return f"'{x!r}'::DOUBLE"


# Student's t at 97.5 % is tabled (t975) to this many degrees of freedom,
# and past them taken from its expansion (t975_expansion).
T975_DF = 100


def incomplete_beta(a, b, x):
    """The regularized incomplete beta I_x(a, b): its continued fraction
    (Lentz's method), from whichever end converges."""
    if x <= 0 or x >= 1:
        return 0.0 if x <= 0 else 1.0
    if x > (a + 1) / (a + b + 2):
        return 1 - incomplete_beta(b, a, 1 - x)
    front = math.exp(math.lgamma(a + b) - math.lgamma(a) - math.lgamma(b) + a * math.log(x) + b * math.log1p(-x))
    tiny = 1e-300
    c, d = 1.0, 1 - (a + b) * x / (a + 1)
    d = 1 / (d if abs(d) > tiny else tiny)
    h = d
    for m in range(1, 1000):
        for num in (m * (b - m) * x / ((a + 2 * m - 1) * (a + 2 * m)),
                    -(a + m) * (a + b + m) * x / ((a + 2 * m) * (a + 2 * m + 1))):
            d = 1 + num * d
            d = 1 / (d if abs(d) > tiny else tiny)
            c = 1 + num / c
            c = c if abs(c) > tiny else tiny
            h *= d * c
        if abs(d * c - 1) < 1e-16:
            break
    return front * h / a


@functools.cache
def t975():
    """Student's t at 97.5 % for each df to T975_DF: the t whose two-sided
    tail, I_{df/(df+t^2)}(df/2, 1/2), is 0.05, by bisection to 1e-13."""
    out = {}
    for df in range(1, T975_DF + 1):
        lo, hi = 0.0, 100.0
        while hi - lo > 1e-13:
            mid = (lo + hi) / 2
            lo, hi = (mid, hi) if incomplete_beta(df / 2, 0.5, df / (df + mid * mid)) > 0.05 else (lo, mid)
        out[df] = (lo + hi) / 2
    return out


def t975_expansion():
    """The normal quantile z at 97.5 % and the coefficients of t's
    expansion in 1/df (Abramowitz and Stegun 26.7.5): t = z + g1/df +
    g2/df^2 + g3/df^3 + g4/df^4."""
    z = statistics.NormalDist().inv_cdf(0.975)
    return z, [(z ** 3 + z) / 4, (5 * z ** 5 + 16 * z ** 3 + 3 * z) / 96,
               (3 * z ** 7 + 19 * z ** 5 + 17 * z ** 3 - 15 * z) / 384,
               (79 * z ** 9 + 776 * z ** 7 + 1482 * z ** 5 - 1920 * z ** 3 - 945 * z) / 92160]


def getter_sql(sql, cols):
    """A metric's SQL with each field of r, v or j that `cols` (by table
    letter: a set of column names, or None for any) lacks read as NULL."""
    def sub(m):
        have = cols.get(m.group(1))
        if have is None or m.group(2) in have:
            return m.group(0)
        return "(NULL::DOUBLE[])" if sql[m.end():m.end() + 1] == "[" else "NULL::DOUBLE"
    return FIELD.sub(sub, sql)


def scoring_sql(cols, temp=False, suite=None):
    """The scoring objects (scoring.sql's header) as SQL: metric_defs and
    job_metrics, from suite.py's METRICS (`suite` the module, by default
    this checkout's), then scoring.sql's tables and macros, its constants
    and row fields filled in. `cols` holds the columns of rows (r),
    verdicts (v) and job_runs (j), so a field they lack reads NULL. With
    `temp` every object is TEMP, a session's own: one whose suite.py
    differs from the catalog's indexer's scores by its own METRICS."""
    s = suite or suite_module()
    defs = []
    for i, (name, sql, better, kind, tie) in enumerate(s.METRICS, 1):
        guard = s.GUARDRAILS.get(name)
        order = list(s.GUARDRAILS).index(name) + 1 if guard else None
        defs.append(f"({i}, {q(name)}, {q(sql)}, {q(better) if better else 'NULL'}, {q(kind)}, {float(tie)!r}, "
                    f"{str(name in s.HOST_METRICS).lower()}, {str(name in s.SHOWN).lower()}, "
                    f"{q(guard) if guard else 'NULL'}, {order if order else 'NULL'})")
    out = ["CREATE OR REPLACE TABLE metric_defs (ord BIGINT, name VARCHAR, getter VARCHAR, better VARCHAR, "
           "kind VARCHAR, tie DOUBLE, host BOOLEAN, shown BOOLEAN, guard VARCHAR, guard_ord BIGINT);",
           f"INSERT INTO metric_defs VALUES {', '.join(defs)};",
           "-- Student's t at 97.5 % by degrees of freedom (catalog.py's t975).",
           "CREATE OR REPLACE TABLE t975 (df BIGINT, t DOUBLE);",
           f"INSERT INTO t975 VALUES {', '.join(f'({df}, {double(t)})' for df, t in t975().items())};"]
    getters = ",\n       ".join(f"TRY_CAST(({getter_sql(sql, cols)}) AS DOUBLE) AS {ident(name)}"
                               for name, sql, *_ in s.METRICS)
    out.append(f"""-- Every source's copy of a job (the catalog's raw.runs): its run's own
-- numbers, which another copy's needn't share (wall_s, rss_mb: its host's).
CREATE OR REPLACE VIEW job_runs AS
SELECT k, src, cell, arm, seed, trace, wall_s, rss_mb FROM raw.runs
QUALIFY row_number() OVER (PARTITION BY k, src ORDER BY cell) = 1;
-- The metric values (metric_defs') of the runs in `runs`, a list of
-- {{'k', 'src'}}, one a metric a run has a value of: k, src (the run's
-- source, the job's own, jobs.src, where NULL), metric, value. A run reads
-- the job's row and verdict (r, v) and its own copy's numbers (j).
CREATE OR REPLACE MACRO job_metrics_of(runs) AS TABLE
SELECT k, src, metric, value FROM (UNPIVOT (
    SELECT u.k, coalesce(u.src, r.src) AS src,
       {getters}
    FROM (SELECT DISTINCT unnest(runs, recursive := true)) u JOIN rows r ON r.k = u.k
    LEFT JOIN verdicts v ON v.k = r.k AND v.src = r.src
    LEFT JOIN job_runs j ON j.k = u.k AND j.src = coalesce(u.src, r.src)
) ON COLUMNS(* EXCLUDE (k, src)) INTO NAME metric VALUE value);
-- Every job's metric values, its own source's (jobs.src), as
-- job_metrics_of gives them.
CREATE OR REPLACE VIEW job_metrics AS
SELECT k, src, metric, value FROM (UNPIVOT (
    SELECT r.k, r.src,
       {getters}
    FROM rows r LEFT JOIN verdicts v USING (k, src) LEFT JOIN job_runs j USING (k, src)
) ON COLUMNS(* EXCLUDE (k, src)) INTO NAME metric VALUE value);""")
    consts = scoring_consts(s)
    text = CONST.sub(lambda m: str(consts[m.group(1)]), SCORING.read_text())
    text = ROW_FIELD.sub(lambda m: f"r.{m.group(1)}" if cols.get("r") is None or m.group(1) in cols["r"]
                         else "NULL::DOUBLE", text)
    out.append(text)
    sql = "\n".join(out)
    return TEMP.sub(lambda m: f"CREATE OR REPLACE TEMP {m.group(1)} ", sql) if temp else sql


def scoring_version(suite=None):
    """A hash of what scoring_sql makes: scoring.sql, METRICS and the
    rest of suite.py's it reads, the constants and scoring_sql itself."""
    s = suite or suite_module()
    parts = [SCORING.read_text(), repr([s.METRICS, s.GUARDRAILS, s.HOST_METRICS, s.SHOWN, scoring_consts(s),
                                        t975()]), inspect.getsource(scoring_sql), inspect.getsource(getter_sql)]
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()[:12]


def scoring_cols(columns):
    """scoring_sql's `cols` from `columns`, {table: its columns}."""
    return {"r": set(columns.get("rows", ())), "v": set(columns.get("verdicts", ())), "j": None}


def scoring_docs():
    """table_catalog's records of scoring.sql's tables and macros, and of
    metric_defs and job_metrics: each one's comment."""
    recs = [{"name": "t975", "kind": "table", "grain": "a degree of freedom", "keys": "df",
             "joins": None,
             "description": "Student's t at 97.5 % (t) for df 1 to T975_DF: the t whose two-sided tail, the "
                            "regularized incomplete beta I_{df/(df+t^2)}(df/2, 1/2), is 0.05. t975_at gives "
                            "any df's."},
            {"name": "metric_defs", "kind": "table", "grain": "a suite.py metric", "keys": "name",
             "joins": "job_metrics ON job_metrics.metric = metric_defs.name",
             "description": "suite.py's METRICS, as the scoring macros read them: ord (its place), getter (its "
                            "value, SQL over a job's row r, verdict v and job j), better ('lower', 'higher' or "
                            "NULL), kind ('ratio' or 'diff'), tie (the band a tie is within), host (the host's "
                            "number, marked h and kept out of totals), shown (a scorecard shows it, ties too), "
                            "guard ('worse', 'lo' or 'hi': how arm_score breaks an arm on it) and guard_ord."},
            {"name": "job_metrics", "kind": "view", "grain": "a job's metric", "keys": "k, metric",
             "joins": "jobs USING (k, src); metric_defs ON metric_defs.name = job_metrics.metric",
             "description": "Each job's value of each metric_defs metric it has one of (value DOUBLE), its own "
                            "source's (jobs.src): the values suite.py's scorecards pair."},
            {"name": "job_metrics_of", "kind": "table macro", "grain": "a run's metric", "keys": "(runs)",
             "joins": "metric_defs ON metric_defs.name = job_metrics_of.metric",
             "description": "The metric values of the runs in `runs`, a list of {'k', 'src'}: k, src (the run's "
                            "source, jobs.src where NULL), metric, value. A run reads its job's row and verdict and "
                            "its own copy's host numbers (job_runs)."},
            {"name": "job_runs", "kind": "view", "grain": "a source's copy of a job", "keys": "k, src",
             "joins": "jobs USING (k)",
             "description": "Every cache source's copy of a job (raw.runs): cell and arm as it stored them, seed, "
                            "trace, and its own run's wall_s and rss_mb, which another copy of the "
                            "job (a rerun, a verify copy) needn't share."}]
    consts = scoring_consts()
    text = CONST.sub(lambda m: str(consts[m.group(1)]), SCORING.read_text())
    header = []
    for line in text.splitlines()[3:]:
        if not line.startswith("--"):
            break
        header.append(line[2:].strip())
    for m in MACRO.finditer(text):
        kind, name, params = m.groups()
        before = text[:m.start()].rstrip("\n").splitlines()
        comment = []
        while before and before[-1].startswith("--"):
            comment.insert(0, before.pop()[2:].strip())
        body = text[m.end():text.find(";", m.end())]
        kind = ("table macro" if " AS TABLE" in body[:200] else "macro") if kind == "MACRO" else kind.lower()
        desc = " ".join(c for c in comment if c) or None
        recs.append({"name": name, "kind": kind, "grain": None, "keys": params and re.sub(r"\s+", " ", params),
                     "joins": None,
                     "description": desc})
    recs.append({"name": "scoring.sql", "kind": "macros", "grain": None, "keys": None, "joins": None,
                 "description": " ".join(h for h in header if h)})
    return recs


# --- maintenance ------------------------------------------------------------------


def compact(keep_minutes=15):
    """Merges the written tables' small files, and lets go of the snapshots
    (and the files only they read) older than `keep_minutes`: a reader's
    query that started before then may lose its files."""
    have = set(lake_columns())
    sql = [f"CALL ducklake_merge_adjacent_files('lake', {q(t)}, schema => 'raw');" for t in WRITTEN if t in have]
    sql += [f"CALL ducklake_expire_snapshots('lake', older_than => now() - INTERVAL {int(keep_minutes)} MINUTE);",
            f"CALL ducklake_cleanup_old_files('lake', older_than => now() - INTERVAL {int(keep_minutes)} MINUTE);"]
    lake("\n".join(sql), write=True, out=None)
    null_stats()


# --- fetch ------------------------------------------------------------------------


def s3_spent(before):
    """What the farm's S3 ledger's lines past byte `before` cost, as farm.py
    prices them, and how many."""
    path = simdb().main_checkout() / "target" / "sim-farm" / "s3.jsonl"
    p = s3_prices()
    total = n = 0
    with contextlib.suppress(OSError):
        with open(path, "rb") as f:
            f.seek(before)
            for line in f.read().decode(errors="replace").splitlines():
                with contextlib.suppress(ValueError):
                    r = json.loads(line)
                    region = r.get("region", "")
                    cross = (sum(v for k, v in (r.get("up") or {}).items() if k not in ("here", region))
                             + sum(v for k, v in (r.get("down") or {}).items() if k not in ("internet", region)))
                    total += (sum((r.get("tier1") or {}).values()) * p.get("S3_TIER1", 0)
                              + sum((r.get("tier2") or {}).values()) * p.get("S3_TIER2", 0)
                              + cross / p.get("GB", 1) * p.get("S3_INTER_REGION_GB", 0)
                              + (r.get("down") or {}).get("internet", 0) / p.get("GB", 1) * p.get("S3_INTERNET_GB", 0)
                              + r.get("stored", 0) / p.get("GB", 1) * p.get("S3_STORAGE_GB_MONTH", 0)
                              * p.get("S3_KEPT_DAYS", 0) / 30)
                    n += 1
    return total, n


def ledger_size():
    with contextlib.suppress(OSError):
        return os.stat(simdb().main_checkout() / "target" / "sim-farm" / "s3.jsonl").st_size
    return 0


def fetch(exp, cells=None, arms=None, tables=(), workers=None, limit_mb=None):
    """Copies what S3 holds of experiment `exp`'s jobs (`cells`, `arms`) into
    the cache, then indexes: the flushes holding their tables (simdb.pull of
    each farm run the cache lacks one of), and with `tables` (of
    SAMPLE_TABLES) the fewest flushes giving the jobs those
    (simdb.sample_plan, fetch_samples). Returns (S3 cost, ledger lines)."""
    sm = simdb()
    if not catalog_path().exists():
        index(workers)
    bad = [t for t in tables if t not in sm.SAMPLE_TABLES]
    if bad:
        sys.exit(f"--tables {', '.join(bad)}: fetch takes the sample tables ({', '.join(sm.SAMPLE_TABLES)}); the "
                 "others come with a flush's sim-data")
    where = [f"exp_id = {q(exp)}"]
    if cells:
        where.append(f"cell IN ({', '.join(q(c) for c in cells)})")
    if arms:
        where.append(f"arm_label IN ({', '.join(q(a) for a in arms)})")
    keys = [r["k"] for r in lake(f"SELECT DISTINCT k FROM experiment_jobs WHERE {' AND '.join(where)};")]
    if not keys:
        sys.exit(f"experiment {exp} has no jobs" + (" of those cells and arms" if cells or arms else "")
                 + ": experiments lists them")
    before = ledger_size()
    with scratch() as tmp:
        jsonl(tmp / "keys.jsonl", [{"k": k} for k in keys])
        holders = lake(f"SELECT DISTINCT f.src FROM raw.farm_index f SEMI JOIN {read_jsonl(tmp / 'keys.jsonl', {'k': 'VARCHAR'})} w "
                       "USING (k) WHERE f.src LIKE 'farm/%';")
        lack = [r["src"] for r in holders if not sm.have_src(r["src"])]
        if lack:
            print(f"{exp}: pulling the farm runs of {len(lack)} flushes the cache lacks", file=sys.stderr)
            gone = sm.pull_sources(lack)
            if gone:
                print(f"{len(gone)} of them are gone from S3: their jobs' tables are left out", file=sys.stderr)
        if tables:
            srcs = sorted({s for s in [r["src"] for r in holders] if sm.have_src(s)} |
                          {r["src"] for r in lake(f"SELECT DISTINCT src FROM raw.runs SEMI JOIN "
                                                  f"{read_jsonl(tmp / 'keys.jsonl', {'k': 'VARCHAR'})} w USING (k);")
                           if sm.have_src(r["src"])})
            v = (f"CREATE VIEW runs_all AS SELECT k, src FROM read_parquet({sm.plist(sm.files(srcs, 'runs'))}, "
                 f"union_by_name=true);\nCREATE VIEW runs AS SELECT k FROM "
                 f"{read_jsonl(tmp / 'keys.jsonl', {'k': 'VARCHAR'})};")
            sm.pull_samples(v, tuple(tables), limit_mb=limit_mb)
    index(workers)
    return s3_spent(before)


# --- commands ---------------------------------------------------------------------


def cmd_index(a):
    index(a.workers, quiet=a.quiet)


def cmd_fetch(a):
    split = lambda s: [x.strip() for x in (s or "").split(",") if x.strip()]
    cost, n = fetch(a.exp, split(a.cells), split(a.arms), split(a.tables), a.workers, a.pull_mb)
    print(f"{a.exp}: fetched; S3 charged ${cost:.4f} ({n} ledger lines)")


def cmd_lake(a):
    if not catalog_path().exists():
        sys.exit(f"no catalog at {catalog_path()}: run simdb.py index")
    out = lake(a.query + ("" if a.query.rstrip().endswith(";") else ";"), out="json" if a.json else "text")
    print(json.dumps(out) if a.json else out, end="" if not a.json else "\n")


def add_commands(sub):
    """simdb.py's index, fetch and lake commands."""
    raw = argparse.RawDescriptionHelpFormatter
    i = sub.add_parser("index", help="bring the catalog up to date (catalog.py)", description=__doc__,
                       formatter_class=raw)
    i.add_argument("--workers", type=int, help="sources derived at once (default: half the cores, at most 8)")
    i.add_argument("--quiet", action="store_true")
    f = sub.add_parser("fetch", help="copy an experiment's data from S3 into the cache, and index it")
    f.add_argument("--exp", required=True, help="an experiment's exp_id (a suite farm run's name)")
    f.add_argument("--cells", help="only these cells, comma-separated")
    f.add_argument("--arms", help="only these arms (arm_label), comma-separated")
    f.add_argument("--tables", help="sample tables too, comma-separated: reads, forecast")
    f.add_argument("--workers", type=int)
    f.add_argument("--pull-mb", type=float, default=None, help="the most of the sample tables it pulls, MB")
    lk = sub.add_parser("lake", help="a query over the catalog, read-only", description=__doc__, formatter_class=raw)
    lk.add_argument("query")
    lk.add_argument("--json", action="store_true")
    return {"index": cmd_index, "fetch": cmd_fetch, "lake": cmd_lake}


if __name__ == "__main__":
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    cmds = add_commands(ap.add_subparsers(dest="cmd", required=True))
    args = ap.parse_args()
    cmds[args.cmd](args)
