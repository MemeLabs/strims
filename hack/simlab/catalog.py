# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' results catalog: every sim result the machine knows of, in one
DuckLake, so that any question is one SQL statement over it.

  index(workers=None, progress=None, wait=True) -> Indexed | None
                 brings the catalog up to date (below); None when `wait`
                 is false and another index holds lake/index.lock (one
                 that waits for it then does only what it left)
  fetch(exp, cells=None, arms=None, tables=(), workers=None, limit_mb=None,
        progress=None) -> Fetched
                 copies what S3 still holds of experiment `exp`'s jobs
                 into the cache, then indexes; what S3 charged for it
  lake(sql, write=False, out="json")
                 a script over the catalog, in a connection of its own:
                 a reader, or the indexer's writer
  connected(write=False)
                 lake's connection, as a block's, for a session of
                 several scripts
  pinned()       this thread's readers opened in the block at one
                 snapshot (below)
  pool(name, size=1, guard=engine.Guard()) -> engine.Pool
                 guarded readers of the catalog (the console's, the Ask
                 eval's, and the CLI's for a query with params, a saved
                 view and describe), spilling under the cache's tmp/<name>/
  schema(pool) -> dict
                 what the console's GET /console/schema answers, from an
                 engine.Pool's readers: {tables (each table's schema,
                 name and columns), table_catalog, metric_catalog,
                 event_kinds, experiments, catalog_info}; the console adds
                 its saved views
  state() -> dict
                 {info: catalog_info, current: whether an index would
                 find nothing to do}, or None without a catalog
  snapshot(lake=None) -> dict
                 {id, at}: the catalog's latest DuckLake snapshot, which
                 every index that changes it moves, or None (below)
  availability(exp=None) -> engine.Result
                 what the catalog holds of each job, table by table (of
                 experiment `exp`'s jobs)
  compact(keep_minutes=15)
                 merges the written tables' small files, and lets go of
                 the snapshots superseded more than keep_minutes ago
  wal() -> bool
                 puts the catalog's metadata in WAL mode (below), as each
                 index does
  scoring_sql(cols, temp=False, suite=None)
                 the scoring objects, as SQL (below)
  scoring_statements(names, have) -> [str]
                 scoring.sql's statements making `names`, for views over
                 the cache (simlab.query's) that take them
  docs_version() -> str
                 a hash of what the doc tables are made from but the
                 catalog's columns (below)
  docs_of(info) -> dict
                 whether this process's docs are main's, which the
                 shared catalog holds (below)

hack/sim-db/simdb.py's index, fetch, lake, view and describe commands
call these (hack/sim-db/catalog.py's), printing an index's and a fetch's
Progress on stderr; the console's server runs index and fetch in a
forkserver child, sending their Progress to its streams. A failure raises
(SystemExit with a CLI's message where one said why): the catalog is then
as it was.

The catalog is a DuckLake (the `ducklake` extension, EXTENSIONS, from
simlab.engine, pins its build for the pinned DuckDB) whose metadata is SQLite,
<cache root>/lake/catalog.sqlite (the cache root is store_host's,
`SIM_DB_ROOT` or the main checkout's target/sim-db), and whose data are
Parquet files under lake/data/. One writer (`index`, engine.writer) and
any number of readers (engine.reader) coexist: a reader attaches it
read-only,

  LOAD ducklake; LOAD sqlite;
  ATTACH 'ducklake:sqlite:<lake>/catalog.sqlite' AS lake (READ_ONLY);
  USE lake;

and each query sees one snapshot, whatever the indexer is writing.
`connected`'s and `lake`'s readers are pinned (SNAPSHOT_VERSION) at the
latest snapshot as they open, so every statement of a script, or of a
session of several on one connection (simlab.scoring's), reads the
catalog as one index left it: the tables, views and macros an index
makes again stay under them. Inside `pinned()` every reader the thread
opens takes the block's one snapshot (the scorecards golden scores an
experiment in one); `index` and `compact` read the latest whatever pin
is set. A snapshot stays until `compact` finds it superseded more than
keep_minutes ago, so a reader on it keeps its tables, and the files they
read, for that long after the next index; one pinned past that fails
(DuckLake: no snapshot at that version) rather than read a catalog whose
files went. Pools' readers (the console's, engine.Pool) aren't pinned:
each query reads the latest as it starts, and the console refreshes them.
`snapshot` reads the latest from the SQLite metadata alone, without
DuckLake: a reader whose snapshot isn't it was opened before an index,
whichever process ran that one (the console refreshes its readers then).

The metadata's journal is WAL (`wal`, at the start of each index, under
its lock): a reader's transaction, which holds the metadata for the
whole of its query, never holds up the writer's commit, nor the commit a
reader, and other processes' connections, whatever code they run, read
and write it as WAL from then on. Every connection simlab opens to the
metadata is the sqlite extension's, one SQLite in the process: a second
(stdlib's sqlite3) lets go of the others' file locks when it closes the
file, and in WAL takes itself for the last connection and removes the
-wal under them. With a rollback journal (a catalog no
index has made WAL yet, or one whose readers kept the switch from its
lock for WAL_SWITCH_S: the next index tries again) a commit waits for
the readers in flight to finish, and the readers that come meanwhile for
the commit. Every connection waits out a lock for up to
engine.BUSY_TIMEOUT_S rather than fail (simlab.engine's docs), and the
index's writes (the transaction, the statistics' flag, compacting) are
tried again after each of WRITE_RETRY_S's waits while the lock refuses
them: a refusal SQLite gives at once, or one that outlasted the timeout.
Compacting that the lock refuses past them is skipped, the index saying
why and succeeding (its data are in); lake/compact.skipped says so until
an index compacts, the next one doing it even with nothing else to do.

`index` brings the catalog up to date with the cache (the suite's and
the farm's flushes), the suite's and the farm's indexes, the farm's costs
and S3 ledger, and git; it does only what changed since its last run, is
idempotent, runs under lake/index.lock, and does nothing but look when
nothing changed. A cache source's sim tables are hard links of the
cache's own Parquet files, registered with the lake
(ducklake_add_data_files), so they cost no disk while the cache holds
them; a file whose column types differ from the lake table's (a column
new to the lake takes the type most of its files give it) is rewritten,
cast. A source gone from the cache (simdb.py gc) leaves the catalog at
the next index, and its files with it. The rows, the verdicts and the
metadata tables are written by the lake, and its small files are merged
at the end of each index (ducklake_merge_adjacent_files); snapshots
superseded more than 15 minutes before, and the files only they read, go
then too. After each
write the tables' statistics are marked as holding NULLs (null_stats).
The verdicts are the derived tables (simlab.verdict's derive_source:
verdict and run_metrics), made for every source that lacks them in a
pool of `workers` processes (verdict.derive).

`fetch` copies what S3 still holds of an experiment's jobs into the cache
(simlab.store's pulls, and with `tables` the bulky sample tables of the
jobs picked by `cells` and `arms`: store.pull_samples), indexes them,
and returns what S3 charged for it, from the farm's S3 ledger.

Progress events are Progress(kind, message, done, total): kind "index"
(the lock, a catalog made again), "scan" (what changed), "derive" (the
verdicts: done of total sources), "write" (the transaction, then
compacting, a write the lock refused and its next try, compacting
skipped), "pull" (a fetch's pulls) or "done"; message is the line the
CLI prints.

The tables (`table_catalog` holds this, by table, for the console and
for questions in plain language; `metric_catalog` holds each column and
metric, `event_kinds` each event kind, the vocabulary's,
crates/strims-wan/vocab.toml (simlab.vocab), with how many of each the
runs hold: an index warns on stderr of a kind the runs hold that it
lacks, and of each table whose columns have no doc of their own,
column_gaps'):

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
  verdicts, verdict_seconds, episodes, derived_events, viewer_phases,
  phase_windows
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
                    spells, dynamics_phases(exp) its lag in each phase,
                    lag_since(exp) the lag against the seconds since each
                    phase window's anchor (phase_windows), all from
                    viewer_lag_of(jobs), the honest viewers' lag each
                    second. table_catalog describes each (kind 'macro' or
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
node_bins, parents, forecast, arrivals, spec_requests and parks to
viewers USING (k, src, node); bins to node_bins USING (k, src, bin);
seconds to verdict_seconds USING (k, src, sec); a viewer's second
(viewer_seconds, a read's or a stall's t_ms // 1000) to
viewer_phases_of(runs) ON (k, src, viewer) with start <= sec < stop, its
phase (viewer_phases', then tail past the verdict's seconds); the request
and HAVE logs
(asks, haves, spec_requests, parks) to the holds in arrivals, as
table_catalog's joins say.

  SELECT e.cell, e.arm_label, count(*) AS runs, sum(v.collapsed::INT) AS collapsed
  FROM experiment_jobs e JOIN verdicts v USING (k, src)
  WHERE e.exp_id = 'sa0755' GROUP BY ALL ORDER BY ALL

  SELECT cell, metric, a_median, b_median, effect, lo, hi, mark
  FROM effects('s8ed1e', 'p4tf', 's8ed1e', 'vouch') WHERE mark <> ' ' ORDER BY cell_ord, ord

`index` makes the scoring objects from simlab.suite's METRICS and
sql/scoring.sql, again whenever either changes (catalog_info's
scoring_version); suite.py loads its own into each session, TEMP. It
makes the doc tables (table_catalog, metric_catalog and event_kinds:
catalogs_sql) with every change it makes, and on their own whenever the
docs they are made from change (catalog_info's docs_version: below),
the counts of what the runs hold kept as they were. The main checkout's
catalog, which every checkout indexes, holds main's docs: an index whose
docs aren't main's (a branch's worktree, a process older than main's
code; docs_of) indexes the data and leaves the doc tables as they are,
saying so, a table or column only it writes undocumented until its code
reaches main (catalog_info's docs_main, docs_behind).

An experiment's `tiers` are suite.py's TIERS (as they are now) its cells
came from: each cell is matched by its name, test and env (a farm cell's
env its index's, else its matrix's jobs', less their seeds and the run's
knob) to the cells of the tiers, and its tier is the one of those
holding the most of the experiment's cells (then the smaller tier, then
the name). A cell whose env no tier's cell of its name has (TIERS changed
since, or --env set more) is in none, nor is one with no env (its index
predates cells' env, and no matrix of its ran).

The physical tables are in schema `raw`, a source's rows each under its
`src`; the views above read them through `jobs`.
"""

import ast
import collections
import contextlib
import dataclasses
import datetime
import functools
import hashlib
import inspect
import json
import math
import os
import re
import shutil
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

from . import engine, query, store, store_host, verdict, vocab
from .engine import EXTENSIONS

ROOT = Path(__file__).resolve().parents[2]

# A change to the catalog's layout: the next index makes it again, whole.
CATALOG_VERSION = 1
# How long S3 keeps a flush (farm.py's S3_KEPT_DAYS).
S3_DAYS = 7
# The env's knobs promoted to jobs' columns: the sims' setting of each
# (SIM_<NAME> in ppspp, STRIMS_SIM_<NAME> in strims-sim).
PROMOTED = {"known_uplink": "KNOWN_UPLINK", "arrivals": "ARRIVALS", "depart": "DEPART", "exp": "EXP"}
# A job's own keys in its env, left out of env_hash: its seed and output.
JOB_KEYS = ("SIM_SEED", "STRIMS_SIM_SEED", "SIM_OUT", "STRIMS_SIM_OUT")
SIMS = verdict.SIM_NAMES
# The sims' tables that are a job's ids and times, not readings: kept as
# they are where -1 elsewhere is no reading.
ID_COLS = {"k", "src", "sec", "bin", "t_ms", "t_us", "node", "viewer", "seq", "stream"}
# The verdicts' tables (verdict.derive's): a source's verdicts and its
# derived tables.
VERDICT_TABLES = ("verdicts", *verdict.DERIVED)
# Written tables whose small files the index merges.
WRITTEN = ("rows", *VERDICT_TABLES, "farm_index", "fields")
NAME = re.compile(r"^[a-z][a-z0-9_]*$")
# How long snapshot() waits out a busy catalog metadata before it gives up.
SNAPSHOT_BUSY_S = 2
# How long the switch to WAL waits for the readers in flight: while it
# waits, readers that come wait for it, and a connection opened by older
# code fails past the sqlite extension's 5 s.
WAL_SWITCH_S = 2
# The waits (s) before each try again of a write the catalog's lock
# refused (the module docs).
WRITE_RETRY_S = (1, 4, 15)
# SQLite's refusals for a lock (SQLITE_BUSY's message, and SQLITE_LOCKED's).
LOCKED = re.compile(r"database (?:table )?is locked", re.I)
# This thread's pinned() snapshot (`version`), if any.
PIN = threading.local()
# snapshot()'s instance, made in each process that reads one (`pid`,
# `conn`), and the lock its reads take.
SNAPSHOTS = {"lock": threading.Lock()}

q, ident = store_host.q, store_host.ident


def simdb():
    """hack/sim-db/simdb.py (simlab.simdb()): the main checkout (its
    main_checkout, which tests patch)."""
    import simlab

    return simlab.simdb()


def lake_root():
    return store_host.cache_root() / "lake"


def catalog_path():
    return lake_root() / "catalog.sqlite"


def data_path():
    return lake_root() / "data"


def spill():
    """The cache's tmp/, where DuckDB spills (as store_host.duck's), or
    None without a cache."""
    tmp = store_host.cache_root() / "tmp"
    if not tmp.parent.exists():
        return None
    tmp.mkdir(exist_ok=True)
    return tmp


@contextlib.contextmanager
def connected(write=False, threads=None, memory=None):
    """A connection to the catalog of its own, closed after the block
    (engine.close: a transaction left open rolls back, and the catalog's
    lock goes with it): a reader (engine.reader: READ_ONLY, pinned at the
    latest snapshot as it opens, or at pinned()'s) unless `write`, the
    indexer's writer (engine.writer: new data under lake/data/, none of it
    inlined), spilling to the cache's tmp/; store_host.DuckError when it
    doesn't attach."""
    tmp = spill()
    try:
        if write:
            conn = engine.writer(lake_root(), threads, memory, tmp)
        else:
            conn = engine.reader(lake_root(), threads=threads, memory=memory, pin=True,
                                 version=getattr(PIN, "version", None))
    except engine.duckdb.Error as e:
        raise store_host.DuckError(str(e)) from None
    try:
        if not write:
            for x in engine.settings_sql(tmp=tmp):
                conn.execute(x)
        yield conn
    finally:
        engine.close(conn)


def lake(sql, write=False, out="json", threads=None, memory=None):
    """Runs the script `sql` over the catalog in a connection of its own
    (connected): what store_host.duck returns for `out`, the last shown
    statement's rows, the -table text of every shown one, or None;
    store_host.DuckError, DuckDB's message, on a failure."""
    with connected(write, threads, memory) as conn:
        return store.results(conn, sql, out)


@contextlib.contextmanager
def pinned():
    """The readers this thread opens in the block (connected's, lake's) at
    one snapshot, the latest as the block starts (an enclosing block's, for
    one inside another); yields its id. store_host.DuckError when the
    catalog can't be read."""
    have = getattr(PIN, "version", None)
    if have is not None:
        yield have
        return
    try:
        conn = engine.connect()
        try:
            PIN.version = engine.latest_snapshot(conn, lake_root())
        finally:
            conn.close()
    except engine.duckdb.Error as e:
        raise store_host.DuckError(str(e)) from None
    try:
        yield PIN.version
    finally:
        PIN.version = None


@contextlib.contextmanager
def latest():
    """This thread's readers at the latest snapshot in the block, a pin
    set aside (an index's and compacting's own reads)."""
    saved = getattr(PIN, "version", None)
    PIN.version = None
    try:
        yield
    finally:
        PIN.version = saved


def locked_out(e):
    """Whether failure `e` is the catalog's lock refusing it."""
    return bool(LOCKED.search(str(e)))


def retried(fn, say=None, what="write"):
    """fn(), a write to the catalog that rolls back whole where it fails,
    tried again after each of WRITE_RETRY_S's waits while the catalog's
    lock refuses it (saying so, as Progress "write", to `say`);
    store_host.DuckError once the waits run out, or at any other
    failure."""
    for wait in (*WRITE_RETRY_S, None):
        try:
            return fn()
        except store_host.DuckError as e:
            if wait is None or not locked_out(e):
                raise
            if say is not None:
                say("write", f"index: the catalog is locked ({what}): trying again in {wait} s")
            time.sleep(wait)


def written(sql, say=None, what="write"):
    """The write script `sql` (lake's, write=True), retried while the lock
    refuses it."""
    return retried(lambda: lake(sql, write=True, out=None), say, what)


def wal():
    """Puts the catalog's SQLite metadata in WAL mode where its journal is
    still rollback's (the module docs), through the sqlite extension, the
    SQLite the lake's connections use, as their own open does it; one whose
    readers in flight keep the lock from it for WAL_SWITCH_S stays as it
    was. Whether it is WAL."""
    try:
        got = store.execute(f"LOAD sqlite; ATTACH {q(str(catalog_path()))} AS lake_meta (TYPE sqlite, "
                            f"JOURNAL_MODE 'wal', BUSY_TIMEOUT {max(1, round(WAL_SWITCH_S * 1000))});\n"
                            "SELECT journal_mode FROM sqlite_query('lake_meta', 'PRAGMA journal_mode');")
    except store_host.DuckError:
        return False
    return bool(got) and got[0]["journal_mode"] == "wal"


def duck(sql):
    """A script over the cache's files (no catalog), in memory: its last
    shown statement's rows."""
    return store.execute(sql)


def extensions():
    """Installs the extensions where missing and checks their builds."""
    try:
        engine.ensure_extensions()
    except engine.PinError as e:
        sys.exit(str(e))


# --- scanning -----------------------------------------------------------------


def iso(ts):
    return datetime.datetime.fromtimestamp(ts, datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%S+00:00")


def scan_cache():
    """The cache's sources: {src: {"files": {table: stat}, "manifest": {...}}},
    a source a dir holding runs.parquet, its tables its Parquet files."""
    root = store_host.cache_root() / "cache"
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
        out[src] = {"files": files, "manifest": store.manifest_of(src)}
    return out


def inputs():
    """The metadata files the catalog reads and the code making its
    tables, {path: (size, mtime_ns, kind)}; the files the doc tables are
    made from are docs_version's."""
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
    for p in (simdb().main_checkout() / "hack" / "sim-farm" / "farm.py", ROOT / "hack" / "simlab" / "suite.py",
              ROOT / "hack" / "simlab" / "verdict.py", Path(__file__)):
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
    return (f"read_json({q(path)}, format='newline_delimited', columns={store_host.struct(cols)}, "
            "ignore_errors=true)")


def tier_cells():
    """suite.py's TIERS as they are now, a record a tier's cell."""
    try:
        suite = suite_module()
    except Exception:
        return []
    return [{"tier": t, "cell": c["name"], "sim": c["sim"], "env": {str(a): str(b) for a, b in c["env"].items()},
             "test": c.get("test")} for t, cells in suite.TIERS.items() for c in cells]


TIER_COLS = {"tier": "VARCHAR", "cell": "VARCHAR", "sim": "VARCHAR", "env": "MAP(VARCHAR, VARCHAR)", "test": "VARCHAR"}

# suite.py, whose module docs describe its metrics and rows.
SUITE_PY = ROOT / "hack" / "sim-suite" / "suite.py"


def suite_module():
    """simlab.suite: the tiers, METRICS and their uses."""
    from . import suite

    return suite


def suite_doc():
    """suite.py's module docs, read rather than run."""
    with contextlib.suppress(OSError, SyntaxError):
        return ast.get_docstring(ast.parse(SUITE_PY.read_text()), clean=False) or ""
    return ""


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
        doc = suite_doc()
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
    (row_comments)."""
    sentences = doc_sentences()
    out = {}
    for n in names:
        for s, pats in sentences:
            if any(p.match(n) for p in pats):
                out[n] = s if len(s) <= 600 else s[:597] + "..."
                break
    return out | {n: text for n, text in row_comments().items() if n not in out}


def row_comments():
    """The comment lines nearest above the first line that writes each
    field into a sim's row with any (strims-sim's measures, the ppspp
    sim's job_row), {field: text}."""
    out = {}
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
    """verdict.run_metrics' docstring, by metric: its lines indented under
    the first (inspect.cleandoc's, whether or not the compiler dedented
    the docstring), a metric's name, then its text."""
    doc = inspect.cleandoc(verdict.run_metrics.__doc__ or "")
    out, cur = {}, None
    for line in doc.splitlines():
        m = re.match(r"^\s{1,8}([a-z][a-z0-9_]*)\s{2,}(.*)$", line)
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
    the comment above it, in simlab.suite, which holds them)."""
    suite = suite_module()
    src = (ROOT / "hack" / "simlab" / "suite.py").read_text().splitlines()
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


def recorded():
    """The catalog's record of what it holds: (info, files, inputs, sources),
    or None without a catalog, or one of another CATALOG_VERSION (one
    without raw.info is none an index made); store_host.DuckError when it
    can't be read (a lock past the busy timeout), which is no reason to
    make it again."""
    if not catalog_path().exists():
        return None
    with connected() as conn:
        rows = lambda sql: store.results(conn, sql)  # noqa: E731
        if not rows("SELECT count(*) AS n FROM information_schema.tables WHERE table_catalog = 'lake' "
                    "AND table_schema = 'raw' AND table_name = 'info';")[0]["n"]:
            return None
        info = {r["key"]: r["value"] for r in rows("SELECT key, value FROM raw.info;")}
        if info.get("catalog_version") != str(CATALOG_VERSION):
            return None
        files = {(r["src"], r["tbl"]): r for r in rows(
            "SELECT src, tbl, path, cache_path, bytes, inode, mtime_ns, how FROM raw.files;")}
        ins = {r["path"]: (r["size"], r["mtime_ns"], r["kind"]) for r in rows("SELECT * FROM raw.inputs;")}
        srcs = {r["src"]: r for r in rows("SELECT src, verdicts FROM raw.sources;")}
    return info, files, ins, srcs


def look(rec):
    """What an index over the catalog's record `rec` (recorded()'s) finds
    to do: the cache's sources and the metadata files as they are, the
    verdicts' version, the sim tables' files gone, changed and new, the
    metadata files changed, the sources lacking verdicts of the version and
    those whose runs to read again."""
    _, have, seen, done_srcs = rec
    cache = scan_cache()
    ins = inputs()
    version = verdict.derived_version()
    now = {(s, t): f for s, c in cache.items() for t, f in c["files"].items()}
    same = lambda a, b: (a["inode"], a["bytes"], a["mtime_ns"]) == (b["inode"], b["bytes"], b["mtime_ns"])  # noqa: E731
    changed = [key for key, f in now.items() if key in have and not same(have[key], f)]
    new = [key for key in now if key not in have]
    return {"cache": cache, "ins": ins, "version": version, "gone": [key for key in have if key not in now],
            "changed": changed, "new": new,
            "ins_changed": {p for p, v in ins.items() if seen.get(p) != v} | {p for p in seen if p not in ins},
            "need_verdicts": sorted(s for s in cache if done_srcs.get(s, {}).get("verdicts") != version),
            "runs_redo": sorted({s for s, t in [*changed, *new] if t == "runs"})}


def state():
    """The catalog's catalog_info, and whether an index would find nothing
    to do (`current`); None without a catalog, or one of another
    CATALOG_VERSION."""
    rec = recorded()
    if rec is None:
        return None
    info = rec[0]
    x = look(rec)
    current = not any(x[k] for k in ("gone", "changed", "new", "ins_changed", "need_verdicts"))
    current = current and info.get("verdict_version") == x["version"]
    if current:
        _, sv = lake_scoring(lake_columns())
        current = (not sv or info.get("scoring_version") == sv) and not docs_of(info)["moved"]
    return {"info": info, "current": current}


def snapshot(lake=None):
    """The latest snapshot of the catalog at `lake` (lake_root() by
    default), {"id": its snapshot_id, "at": its time as iso writes it}:
    every index that changes the catalog (its sources, metadata, verdicts
    or scoring objects) moves it, whichever process runs the index, and
    one with nothing to do doesn't. It is one row of the SQLite metadata,
    read through the sqlite extension, attached read-only for that read
    alone, on one in-memory DuckDB instance this process keeps (cheap
    enough for every request): the SQLite the lake's own connections use,
    since a second SQLite in the process (stdlib's sqlite3) lets go of
    their file locks when it closes the file, and in WAL takes itself for
    the file's last connection and checkpoints and removes the -wal under
    them. None without a catalog, and while the metadata stays busy past
    SNAPSHOT_BUSY_S (a writer's commit, while its journal is rollback's:
    wal) or can't be read: callers keep the last one they read."""
    path = engine.catalog_file(lake_root() if lake is None else lake)
    if not path.is_file():
        return None
    with SNAPSHOTS["lock"]:
        try:
            if SNAPSHOTS.get("pid") != os.getpid():
                SNAPSHOTS.update(pid=os.getpid(), conn=engine.connect())
            conn = SNAPSHOTS["conn"]
            conn.execute(f"ATTACH {q(str(path.resolve()))} AS lake_snapshot (TYPE sqlite, READ_ONLY, "
                         f"BUSY_TIMEOUT {max(1, round(SNAPSHOT_BUSY_S * 1000))})")
            try:
                rows = conn.execute("SELECT snapshot_id, snapshot_time FROM lake_snapshot.ducklake_snapshot "
                                    "ORDER BY snapshot_id DESC LIMIT 1").fetchall()
            finally:
                conn.execute("DETACH lake_snapshot")
        except (engine.duckdb.Error, engine.PinError):
            return None
    if not rows:
        return None
    n, at = rows[0]
    try:
        at = iso(datetime.datetime.fromisoformat(at).timestamp())
    except (TypeError, ValueError):
        pass
    return {"id": n, "at": at}


def availability(exp=None):
    """The catalog's availability (each job's tables: local, on S3 or
    expired, their source and bytes), of experiment `exp`'s jobs alone
    when given, as an engine.Result."""
    sql = "SELECT * FROM availability ORDER BY k, tbl"
    if exp is not None:
        sql = ("SELECT a.* FROM availability a SEMI JOIN (SELECT k FROM experiment_jobs WHERE exp_id = $exp) e "
               "USING (k) ORDER BY k, tbl")
    with connected() as conn:
        try:
            return engine.run(conn, sql, {"exp": exp} if exp is not None else None)
        except engine.QueryError as e:
            raise store_host.DuckError(str(e)) from None


def wipe():
    """Removes the catalog, for a CATALOG_VERSION it wasn't made under."""
    for p in (catalog_path(), catalog_path().with_name(catalog_path().name + "-wal"),
              catalog_path().with_name(catalog_path().name + "-shm"), skipped_path()):
        p.unlink(missing_ok=True)
    shutil.rmtree(data_path(), ignore_errors=True)


def schemas(paths):
    """The Parquet files' columns, {path: {name: DuckDB type}}."""
    out = collections.defaultdict(dict)
    paths = list(paths)
    for i in range(0, len(paths), 500):
        part = paths[i:i + 500]
        for r in duck(f"SELECT file_name AS f, name, duckdb_type AS t FROM parquet_schema("
                      f"{verdict.plist(part)}) WHERE duckdb_type IS NOT NULL ORDER BY column_id;"):
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


@dataclasses.dataclass
class Progress:
    """A step of an index or a fetch: its kind (the module docs), the line
    the CLI prints, and how far it is (`done` of `total`, where counted)."""
    kind: str
    message: str
    done: int = 0
    total: int = 0


@dataclasses.dataclass
class Indexed:
    """What an index did: whether it changed the catalog (only its doc
    tables, `docs`, or its scoring objects, `scoring`, or both), and
    whether it left the doc tables as main's, its own docs not
    (`docs_kept`: docs_of); the cache sources whose files it added,
    replaced or let go, and how many files each way; the sources it judged
    (their verdicts made, or found made), and those whose verdicts failed,
    {src: why} (they wait for the next index); why it skipped compacting
    (the catalog's lock refused it: the next index compacts), or None."""
    changed: bool = False
    sources: list = dataclasses.field(default_factory=list)
    judged: list = dataclasses.field(default_factory=list)
    failed: dict = dataclasses.field(default_factory=dict)
    new: int = 0
    replaced: int = 0
    gone: int = 0
    scoring: bool = False
    docs: bool = False
    docs_kept: bool = False
    elapsed_s: float = 0.0
    compact_skipped: str = None


@dataclasses.dataclass
class Fetched:
    """What a fetch did: the flushes it pulled, those gone from S3 (to
    regenerate), and what S3 charged (`ledger_lines` lines of
    target/sim-farm/s3.jsonl, priced as farm.py prices them)."""
    exp: str
    s3_usd: float = 0.0
    ledger_lines: int = 0
    pulled: list = dataclasses.field(default_factory=list)
    gone: list = dataclasses.field(default_factory=list)


def sayer(progress):
    """say(kind, message, done=0, total=0): a Progress to `progress`, if
    any."""
    def say(kind, message, done=0, total=0):
        if progress is not None:
            progress(Progress(kind, message, done, total))
    return say


def index(workers=None, progress=None, wait=True):
    """Brings the catalog up to date (the module docs); returns what it did,
    or None when `wait` is false and another index holds the lock."""
    say = sayer(progress)
    t0 = time.monotonic()
    took = lambda: time.monotonic() - t0  # noqa: E731
    lake_root().mkdir(parents=True, exist_ok=True)
    with latest(), store_host.locked(lake_root() / "index.lock", block=wait) as mine:
        if not mine:
            say("index", "another index holds lake/index.lock")
            return None
        extensions()
        rec = recorded()
        fresh = rec is None
        if rec is None:
            if catalog_path().exists():
                say("index", f"the catalog isn't CATALOG_VERSION {CATALOG_VERSION}'s: making it again")
            wipe()
            data_path().mkdir(parents=True, exist_ok=True)
            # A cache source's runs and scalars hold the store's columns,
            # and an older one may lack some.
            own = [f"CREATE TABLE raw.{t} ({', '.join(f'{ident(n)} {ty}' for n, ty in c.items())});"
                   for t, c in (("runs", store_host.RUN_COLS), ("scalars", store_host.SCALAR_COLS))]
            lake(BASE_SQL + "\n".join(own), write=True, out=None)
            rec = ({}, {}, {}, {})
        wal()
        info, have, seen, _ = rec
        x = look(rec)
        cache, ins, version = x["cache"], x["ins"], x["version"]
        gone, changed, new, ins_changed = x["gone"], x["changed"], x["new"], x["ins_changed"]
        need_verdicts = x["need_verdicts"]
        docs = docs_of(info) | {"info": info, "fresh": fresh}
        kept = not docs["own"] and docs["version"] != info.get("docs_version")
        if not (gone or changed or new or ins_changed or need_verdicts) and info.get("verdict_version") == version:
            cols = lake_columns()
            scoring, sv = lake_scoring(cols)
            moved = {"scoring_version": sv} if sv and info.get("scoring_version") != sv else {}
            # The doc tables go with the scoring objects (scoring_docs), as
            # this process's docs only where they are main's.
            remade = docs["own"] and (docs["moved"] or bool(moved))
            if remade:
                moved |= {k: v for k, v in (("docs_version", docs["version"]), ("docs_main", docs["main"])) if v}
            if kept:
                say("index", docs_kept(docs))
            if not moved:
                skipped = owed(say)
                say("done", f"the catalog is up to date ({took():.1f} s)")
                return Indexed(docs_kept=kept, elapsed_s=took(), compact_skipped=skipped)
            # Only the docs or the scoring objects changed: the doc tables,
            # the scoring objects where theirs did, and the versions, alone;
            # the events counted again where an index left the doc tables
            # behind the data.
            gone_keys = [*moved, *(["docs_behind"] if remade else [])]
            with scratch() as tmp:
                written("\n".join(["BEGIN;", *(catalogs_sql(cols, tmp, recount=bool(info.get("docs_behind")))
                                               if remade else []),
                                   *(scoring if "scoring_version" in moved else []),
                                   f"DELETE FROM raw.info WHERE key IN ({', '.join(q(k) for k in gone_keys)});",
                                   "INSERT INTO raw.info VALUES "
                                   + ", ".join(f"({q(k)}, {q(v)})" for k, v in moved.items()) + ";", "COMMIT;"]),
                        say, "the docs")
            warn_undocumented()
            skipped = owed(say)
            what = " and ".join(w for w, on in (("the scoring macros", "scoring_version" in moved), ("the docs", remade))
                                if on)
            say("done", f"index: {what} made again ({', '.join(f'{k} {v}' for k, v in moved.items())}; "
                f"{took():.1f} s)")
            return Indexed(changed=True, scoring="scoring_version" in moved, docs=remade, docs_kept=kept,
                           elapsed_s=took(), compact_skipped=skipped)
        say("scan", f"index: {len(new)} new files, {len(changed)} changed, {len(gone)} gone; "
            f"{len(ins_changed)} metadata files changed; verdicts for {len(need_verdicts)} sources")
        kept = not docs["own"] and not fresh
        if not docs["own"]:
            say("index", docs_kept(docs) if kept else
                f"index: the doc tables made from this checkout's docs (docs_version {docs['version']}), which "
                "aren't main's: there were none to keep")
        derived = verdict.derive(need_verdicts, workers, lambda m, done, total: say("derive", m, done, total))
        failed = derived.failed
        with scratch() as tmp:
            kind_of = {p: v[2] for p, v in seen.items()} | {p: v[2] for p, v in ins.items()}
            sql = build(cache, have, gone, changed, new, x["runs_redo"], ins, ins_changed, kind_of,
                        [s for s in need_verdicts if s not in failed], version, tmp, docs, failed)
            say("write", f"index: writing ({took():.0f} s in)")
            try:
                written(sql, say, "the transaction")
            except store_host.DuckError as e:
                # The transaction rolled back: the catalog is as it was. The
                # SQL and the files it read stay in lake/failed/.
                failed_dir = lake_root() / "failed"
                shutil.rmtree(failed_dir, ignore_errors=True)
                shutil.copytree(tmp, failed_dir)
                (failed_dir / "index.sql").write_text(sql.replace(str(tmp), str(failed_dir)))
                sys.exit(f"index: {e}\n(the SQL: {failed_dir / 'index.sql'}; the catalog is as it was)")
        try:
            retried(null_stats, say, "the statistics' NULLs")
        except store_host.DuckError as e:
            # Compacting marks them too, or the next index's.
            if not locked_out(e):
                raise
        warn_undocumented()
        say("write", f"index: compacting ({took():.0f} s in)")
        skipped = compacted(say)
        say("done", f"index: done in {took():.0f} s")
        return Indexed(changed=True, sources=sorted({s for s, _ in [*new, *changed, *gone]}), judged=derived.judged,
                       failed=failed, new=len(new), replaced=len(changed), gone=len(gone), docs_kept=kept,
                       elapsed_s=took(), compact_skipped=skipped)


def build(cache, have, gone, changed, new, runs_redo, ins, ins_changed, kind_of, need_verdicts, version, tmp, docs,
          failed=()):
    """The index's one transaction, as SQL; `docs` main_sql's."""
    cols = lake_columns()
    pre, sql = [], ["BEGIN;"]
    # The store's runs columns, which the jobs table reads, before any file
    # holds the ones the store gained.
    for n, ty in store_host.RUN_COLS.items():
        if "runs" in cols and n not in cols["runs"]:
            sql.append(f"ALTER TABLE raw.runs ADD COLUMN {ident(n)} {ty};")
            cols["runs"][n] = ty
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
    for t in VERDICT_TABLES:
        if t in cols:
            sql.append(f"DELETE FROM raw.{t} WHERE src NOT IN (SELECT src FROM raw.sources);")
    sql.append("DELETE FROM raw.inputs;")
    sql.append("INSERT INTO raw.inputs BY NAME SELECT * FROM " + read_jsonl(
        jsonl(tmp / "inputs.jsonl", [{"path": p, "size": a, "mtime_ns": b, "kind": c} for p, (a, b, c) in ins.items()]),
        {"path": "VARCHAR", "size": "BIGINT", "mtime_ns": "BIGINT", "kind": "VARCHAR"}) + ";")
    sql += main_sql(cols, version, tmp, ins, parsed, docs, run_commits(cache, add, types))
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
    store.execute(f"LOAD sqlite; ATTACH {q(str(catalog_path()))} AS lake_meta (TYPE sqlite, "
                  f"BUSY_TIMEOUT {engine.busy_ms()});\n"
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
        for r in duck(f"SELECT src, list(DISTINCT sim ORDER BY sim) AS sims FROM read_parquet("
                      f"{verdict.plist(part)}, union_by_name=true) GROUP BY src;"):
            out[r["src"]] = sorted({SIMS.get(x, x) for x in r["sims"] if x})
    return out


def run_commits(cache, keys, types):
    """The commits the runs among `keys` name (their sources' runs files
    with a commit column, by `types`, schemas' of them)."""
    paths = [cache[s]["files"][t]["path"] for s, t in keys if t == "runs"]
    paths = [p for p in paths if "commit" in types.get(p, {})]
    out = set()
    for i in range(0, len(paths), 500):
        out |= {r["c"] for r in duck(f"SELECT DISTINCT commit AS c FROM read_parquet("
                                     f"{verdict.plist(paths[i:i + 500])}, union_by_name=true) "
                                     "WHERE commit IS NOT NULL;")}
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
        got = duck(f"SELECT json_group_structure(row_json::JSON) AS s FROM read_parquet("
                   f"{verdict.plist(paths[i:i + 200])}, union_by_name=true) WHERE row_json IS NOT NULL;")
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
    """The verdict tables of `srcs` under `version`, from verdict.derive's
    files; versions but the current and the one before leave."""
    out = []
    tables = {**{t: dict(c) for t, c in verdict.DERIVED.items()}, "verdicts": dict(verdict.VERDICT_COLS)}
    for t, base in tables.items():
        files = [str(verdict.derived_dir(t) / (s.replace("/", "~") + ".parquet")) for s in srcs]
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
                       f"FROM read_parquet({verdict.plist(files)}, union_by_name=true, hive_partitioning=false);")
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
        for n, ty in spec.items():
            if n not in cols[name]:
                out.append(f"ALTER TABLE raw.{name} ADD COLUMN {ident(n)} {ty};")
                cols[name][n] = ty

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
    tables = set(store_host.TABLES)
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
           r."at", r.bin_ms, json_extract_string(r.row_json, '$.reads_hash') AS reads_hash, r.commit,
           r.binary_sha256, r.dirty,
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
           c.commit AS ccommit, c.binary_sha256 AS cbinary, c.dirty AS cdirty, fi.f,
           coalesce(fi.f.env, json_transform(c.env, '"MAP(VARCHAR, VARCHAR)"')) AS env
    FROM (SELECT k FROM canon UNION SELECT k FROM fi) keys LEFT JOIN canon c USING (k) LEFT JOIN fi USING (k)),
s AS (
    SELECT *, coalesce({csim},
                       CASE WHEN len(list_filter(map_keys(env), lambda x: x LIKE 'STRIMS_SIM_%')) > 0 THEN 'strims-sim'
                            WHEN len(list_filter(map_keys(env), lambda x: x LIKE 'SIM_%')) > 0 THEN 'ppspp' END) AS nsim
    FROM j)
SELECT k, nsim AS sim, coalesce(ctest, f.test) AS test,
       coalesce(cseed, TRY_CAST(env[CASE WHEN nsim = 'ppspp' THEN 'SIM_SEED' ELSE 'STRIMS_SIM_SEED' END] AS BIGINT)) AS seed,
       coalesce(f.commit, ccommit) AS commit, CASE WHEN f.commit IS NOT NULL THEN false ELSE cdirty END AS dirty,
       coalesce(f.binary_sha256, cbinary) AS binary_sha256, env,
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
cell_tiers AS (
    SELECT e.exp_id, e.cell, t.tier, t.size, count(*) OVER (PARTITION BY e.exp_id, t.tier) AS held
    FROM main.experiment_cells e
    JOIN (SELECT *, count(*) OVER (PARTITION BY tier) AS size FROM raw.tier_cells) t
      ON t.cell = e.cell AND (e.test IS NULL OR t.test = e.test)
     AND list_sort(map_entries(t.env)) = list_sort(map_entries(e.env))),
tiers AS (
    SELECT exp_id, list(DISTINCT tier ORDER BY tier) AS tiers FROM (
        SELECT exp_id, cell, arg_min(tier, (-held, size, tier)) AS tier FROM cell_tiers GROUP BY ALL)
    GROUP BY exp_id),
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
SELECT * FROM (
    SELECT src AS exp_id, 'local' AS kind, min(TRY_CAST("at" AS TIMESTAMPTZ)) AS started_at,
           max(TRY_CAST("at" AS TIMESTAMPTZ)) AS finished_at, list(DISTINCT seed ORDER BY seed) AS seeds,
           list(DISTINCT arm ORDER BY arm) AS arm_labels, false AS pending, count(DISTINCT cell) AS cells,
           count(*) AS jobs
    FROM raw.runs WHERE src LIKE 'suite/%' GROUP BY src)
LEFT JOIN (
    SELECT src AS exp_id, map_from_entries(list({{'key': arm, 'value': commit}} ORDER BY arm)) AS arms
    FROM (SELECT src, arm, arg_max(commit, "at") AS commit FROM raw.runs
          WHERE src LIKE 'suite/%' AND arm IS NOT NULL AND commit IS NOT NULL GROUP BY src, arm)
    GROUP BY src) USING (exp_id);
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


def view_sql(t, cols, name, viewers, arrivals=None):
    """The view `name` of sim table `t` (its columns `cols`), normalized as
    the module docs say; `viewers` the viewers table's columns, for the
    viewers' stays, and `arrivals` the arrivals table's, for the asks'
    truth (simlab.query's asks_view)."""
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
    if t == "asks":
        return query.asks_view(f"main.{ident(name)}", f"(SELECT x.* {canon(t)})",
                               f"(SELECT x.* {canon('arrivals')})", cols, arrivals or {})
    if t == "haves":
        return query.haves_view(f"main.{ident(name)}", f"(SELECT x.* {canon(t)})",
                                f"(SELECT x.* {canon('arrivals')})", cols, arrivals or {})
    return f"{head} x.* {canon(t)};"


# The catalog's own names, which a sim table's view doesn't take (it is
# sim_<table> then).
OWN = {"experiments", "experiment_cells", "experiment_jobs", "exp_runs", "jobs", "commits", "costs", "s3",
       "availability", "rows", "scalars", "catalog_info", "metric_catalog", "event_kinds", "table_catalog",
       *VERDICT_TABLES, *(f"{t}_all" for t in VERDICT_TABLES)}
# The raw tables that aren't sims' tables.
META = {"sources", "files", "fields", "inputs", "info", "runs", "rows", "scalars", "farm_index", "suite_runs",
        "suite_cells", "matrices", "costs", "s3_ledger", "bin_hashes", *VERDICT_TABLES, "tier_cells"}


def sim_tables(cols):
    """The raw tables that are sims' tables, {table: its view's name}."""
    return {t: (t if t not in OWN else f"sim_{t}") for t in sorted(cols) if t not in META}


def main_sql(cols, version, tmp, ins, parsed, docs, commits=()):
    """The main tables, made again, and the views; the doc tables as
    `docs` (docs_of's, with the catalog_info it read, `info`, and whether
    the catalog is new, `fresh`) has them made."""
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
    out += commits_sql(tmp, ins, parsed, commits)
    for t, name in sim_tables(cols).items():
        out.append(view_sql(t, cols[t], name, cols.get("viewers", {}), cols.get("arrivals", {})))
    for t in ("rows", "scalars"):
        if t in cols:
            out.append(f"CREATE OR REPLACE VIEW main.{t} AS SELECT x.* {canon(t)};")
    for t in VERDICT_TABLES:
        if t in cols:
            out.append(f"CREATE OR REPLACE VIEW main.{t}_all AS SELECT x.* {canon(t)};")
            out.append(f"CREATE OR REPLACE VIEW main.{t} AS SELECT * FROM main.{t}_all "
                       f"WHERE verdict_version = (SELECT value FROM raw.info WHERE key = 'verdict_version');")
    info = {"catalog_version": str(CATALOG_VERSION), "verdict_version": version,
            "analyze_verdict_version": verdict.VERDICT_VERSION, "duckdb_version": store_host.DUCKDB_VERSION,
            "ducklake_version": EXTENSIONS["ducklake"], "sqlite_version": EXTENSIONS["sqlite_scanner"],
            "indexed_at": iso(time.time())}
    scoring, sv = lake_scoring(cols)
    if sv:
        info["scoring_version"] = sv
    # The doc tables: this process's docs where they are main's, or where
    # there are none to keep; else kept, behind the data.
    make = docs["own"] or docs["fresh"]
    if make and docs["version"]:
        info["docs_version"] = docs["version"]
    if make and docs["own"] and docs["main"]:
        info["docs_main"] = docs["main"]
    if not make:
        info |= {k: docs["info"][k] for k in ("docs_version", "docs_main") if k in docs["info"]}
        info["docs_behind"] = "1"
    out.append("DELETE FROM raw.info;")
    out.append("INSERT INTO raw.info VALUES " + ", ".join(f"({q(k)}, {q(v)})" for k, v in info.items()) + ";")
    # The catalogs first: the scoring macros read event_kinds.
    if make:
        out += catalogs_sql(cols, tmp)
    out += scoring
    return out


def lake_scoring(cols):
    """The catalog's scoring objects (scoring_sql, from this checkout's
    suite.py) over the raw tables' columns `cols`, and their
    scoring_version; none, with a warning, when suite.py doesn't load.
    The views main_sql makes of them are the database's tables (have), so
    a macro that needs one the catalog lacks fails saying so when called
    (needed): DuckDB binds a table macro as it makes it, and one reading a
    table that isn't there would fail the index."""
    have = {name: set(cols[t]) for t, name in sim_tables(cols).items()} | {t: set(c) for t, c in cols.items()}
    try:
        sv = scoring_version()
        return [scoring_sql(scoring_cols({t: cols.get(t, {}) for t in ("rows", "verdicts")}, have=have))], sv
    except Exception as e:
        print(f"index: no scoring macros: suite.py's METRICS don't load ({e})", file=sys.stderr)
        return [], None


def commits_sql(tmp, ins, parsed, runs=()):
    """main.commits: the commits the farm's indexes, the suite's and the
    local runs' (`runs`, those of the sources this index adds) name, from
    git: subject, author, date, whether main holds it, and the local
    branches whose tip it is."""
    shas = set(runs)
    for t in ("farm_index", "runs"):
        # Before the catalog holds the table (or runs their commit column)
        # its query fails.
        with contextlib.suppress(store_host.DuckError):
            shas |= {r["c"] for r in lake(f"SELECT DISTINCT commit AS c FROM raw.{t} WHERE commit IS NOT NULL;")}
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


def either(names):
    """`names` as a list in words: "a, b or c"."""
    names = list(names)
    return names[0] if len(names) == 1 else f"{', '.join(names[:-1])} or {names[-1]}"


def clause(text):
    """A description as a clause: its first letter lower case (but an
    acronym's), its full stop dropped."""
    text = text.strip().rstrip(".")
    if len(text) > 1 and text[0].isupper() and not text[1].isupper():
        text = text[0].lower() + text[1:]
    return text


def verdict_phases():
    """The phases the verdict gives a second: the vocabulary's, but
    query.TAIL (a read's past the verdict's seconds)."""
    return [p for p in vocab.phases() if p != query.TAIL]


def verdict_phases_text():
    """The verdict's phases, each with its description."""
    return either(f"{p} ({clause(vocab.entries('phases')[p]['description'])})" for p in verdict_phases())


def derived_text():
    """The derived kinds, each with what its value holds."""
    return ", ".join(kid + (f" (value: {v})" if (v := vocab.value_text(vocab.kind(kid))) else "")
                     for kid in vocab.derived())


# The catalog's tables: (kind, grain, keys, joins, description). The sims'
# tables add their own, from the sims' docs (doc_tables); the vocabulary
# gives the kinds and phases.
TABLE_DOCS = {
    "experiments": (
        "table", "an experiment", "exp_id", "experiment_cells, experiment_jobs USING (exp_id)",
        "A suite farm run (kind 'farm'; exp_id its name, as suite.py farm printed it) or a local suite invocation "
        "(kind 'local'; exp_id its cache source, suite/<worktree>/<invocation>, which holds the jobs it ran itself, "
        "not the ones it kept from earlier invocations). started_at and finished_at: its own jobs' first and last "
        "ran_at (a farm run's pending index's time before any ran); tiers (a farm run's): suite.py's TIERS, as they "
        "are now, its cells came from, each cell's the tier holding the most of its cells among those with a cell of "
        "its name, test and env (none for a cell whose env no tier's has); seeds; knob (the env variable by sim) and "
        "sets of a --sets run; arms (name: commit; a farm run's from its index, a local one's its jobs' by label) "
        "and arm_labels (experiment_jobs' arms); base: the arm the others are scored against, where its index names "
        "it (a --sets run's first set else); question "
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
        "Every job the cache or the farm's indexes know: sim ('ppspp' or 'strims-sim'), test, seed, commit (the "
        "one its binary was built from: a farm job's from its index, a local one's from its run), dirty (whether "
        "the working tree's changes to that commit's build inputs went into the binary; false for the farm's, "
        "which builds commits alone) and binary_sha256, env (its settings, MAP) and knobs promoted from it "
        "(transport: 'tcp' or 'webrtc' (rtc_browsers the browsers) or NULL for the sim's own model; known_uplink, "
        "arrivals, depart and exp: the sims' SIM_*/STRIMS_SIM_* settings), env_hash (its settings but seed and "
        "output, so equal settings match across experiments), trace and reads_hash (the digests a rerun must "
        "reproduce), wall_s and rss_mb (its host's), ran_at, and src: the cache source its tables come from "
        "(simdb.py's source_ranks: the one holding the most tables, then the later flush, then the later seq), or "
        "the farm flush that holds them when the cache doesn't (cached false). tables: the tables src holds; "
        "stored_cell and stored_arm: the cell (a farm matrix, or a local cell) and arm its source stored it under; "
        "bin_ms: its bins' width."),
    "commits": (
        "table", "a commit", "sha", "jobs ON jobs.commit = commits.sha; experiments' arms values",
        "The commits the jobs and the experiments' arms name, from git: subject, author, committed_at, "
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
        f"'sim-data' (its flush's tables) and, where its samples landed, 'sim-samples' "
        f"({', '.join(store_host.SAMPLE_TABLES)})."),
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
        f"Each second's verdict: phase, the vocabulary's: {verdict_phases_text()}; healthy and what judged it "
        "(trees_bad, deliv_bad, lag_high, lag_rise), hit_share, delivery and lag_q (the honest viewers' q0.9 lag)."),
    "episodes": (
        "view", "a job's spell", "k, kind, start", "jobs USING (k, src)",
        "A run's healthy, unhealthy and collapse spells: [start, stop) seconds, len."),
    "derived_events": (
        "view", "a job's derived event", "k, sec, kind", "jobs USING (k, src)",
        f"analyze.events, the vocabulary's derived kinds (event_kinds' sim 'derived'): {derived_text()}."),
    "viewer_phases": (
        "view", "a viewer's span of one phase", "k, viewer, start",
        "jobs USING (k, src); viewer_seconds, reads (t_ms // 1000) and stalls ON (k, src, viewer) with start <= "
        "sec < stop",
        "Each viewer's phases (analyze.viewer_phases), which viewer_phases_of(runs) reads a viewer-second's phase "
        "from (tail past the verdict's seconds): [start, stop) "
        "seconds in phase, the seconds it was there any part of, within the verdict's seconds: each incarnation's "
        "from its join's second (0 for a viewer there from the start) to its departure's, leave's or crash's. A "
        "second's phase is the window that holds it of the run's and the viewer's own joins' (phase_windows; the "
        "verdict's rule, analyze.timeline): startup while its own join's window holds it (each incarnation's), "
        "else the run's verdict_seconds phase. Every viewer's, the hostile too (viewers.hostile)."),
    "phase_windows": (
        "view", "a job's phase window", "k, anchor_s, kind", "jobs USING (k, src); viewer_phases ON (k, src) with "
        "phase and anchor_s <= viewer_phases.start + 0.5 < stop_s",
        "The windows the verdict's phases come from (analyze.timeline), unresolved, to explain a phase: one for "
        "the stream's start (kind start, the run's startup), then one for each event of a kind that anchors a "
        "phase (event_kinds' scenario kinds; the vocabulary's phase, until and horizon_s), at t > 0: kind, node "
        "(NULL for the start's), viewer (a join's viewer, whose own startup it is; NULL for a window of the run's), "
        "phase (a level's, uplink_rate's or rate_limit's, after_down or after_up by its value against the node's "
        "previous level), anchor_s (its event's time), end_s (a span's end: its until event's time, NULL without "
        "one or for a step), stop_s (when it stops holding seconds: anchor_s, or end_s or the run's end for a "
        "span, plus horizon_s) and horizon_s. A second k is in a window when anchor_s <= k + 0.5 < stop_s; of "
        "those holding it, the one whose latest event (end_s once it came, else anchor_s) is latest wins, a tie "
        "to the kind the vocabulary lists first. A series without events implies its windows: the viewers' "
        "joins and departures from their stays, the seed's uplink levels from seed_up."),
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
        "Every kind of the vocabulary (crates/strims-wan/vocab.toml), a row for each sim whose runs may hold it "
        "(sim 'derived' for derived_events'; both sims for a kind only other builds write, under its knob), and "
        "each kind a sim's events hold that the vocabulary doesn't give that sim (documented false, the rest "
        f"NULL): class ({either(vocab.CLASSES)}), scope (whose seconds a scenario kind bears on: run, every "
        "viewer's, or viewer, its node's viewer's alone, as a join's), description, what node, peer, value "
        "(value_meaning, its unit after it) and detail hold, details (detail's values), notes (that sim's, where "
        "its kind differs), env_gate (the knob a sim writes it under), replaced_by (a retired kind's current id "
        "for that sim: a retired kind's events of that sim read as that kind's), documented, n (how many events: "
        "0 for a kind no run has written), and when first and last seen."),
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
    "events": "t_ms = t_us / 1000; node and peer NULL for none; a retired parent_lost's value NULL if the viewer "
              "never stopped counting on the parent; detail_map: detail as a MAP where it is key=value pairs.",
    "seconds": "-1 (no reading) NULL.", "bins": "-1 (no reading) NULL.",
    "node_bins": "viewer NULL for the seed and hostile hosts.",
    "stalls": "The unit played is the sim's: a frame in strims-sim, a chunk in the ppspp sim, whose stalls are "
              "one chunk's lateness and so often far shorter than a frame's. A unit read twice counts at its "
              "first read. macros.sql's playback(delay_ms) plays reads the same way at any delay.",
    "parents": "parent NULL for none. Only hybrid push gives a stream push parents: empty in other modes.",
    "forecast": "horizon_ms, held_ms and mean_ms NULL where -1 (no estimate, never held).",
    "asks": "Every rival kept, in both rounds: a row per request and channel it weighed, the asked row (picked) "
            "first; grace_ms, outcome, reason and t_outcome_ms are the asked row's, NULL on a rival's. The sims "
            "write what a request was decided on (peer_honest the peer's at the request); the view adds its "
            "truth from arrivals, as spec_requests' columns are: held_ms, the peer's first hold of the chunk in "
            "its incarnation then (arrivals' earliest row of node = peer, seq, incarnation = peer_incarnation; "
            "NULL if none), and from it live (held by since_ms and channel_open_ms, in whole ms), err_ms, "
            "pred_err_ms, held_by_p50, held_by_p90 and held_by_p99. Main's (the HAVE world's request log) "
            "has one round, 'flight'; its predicted_ms and predicted_p90_ms are a rival's route expectation "
            "(first sight plus its announcement lag's mean, plus its horizon), NULL on the asked row, which a "
            "HAVE decided; timeout_ms, have_ms (the row's peer's HAVE of the chunk) and released_ms (push's "
            "release of it to pull) are its own; grace_ms, frontier_seq, hole_seq and state_age_ms are NULL "
            "there.",
    "haves": "Main's HAVE log: a row per chunk a HAVE first announced on a channel. The sims write the HAVE "
             "(haves_log); the view adds held_ms, the announcing peer's first hold of the chunk in its "
             "incarnation then (arrivals' earliest row of node = peer, seq, incarnation = peer_incarnation; NULL "
             "if none), delay_ms (t_ms - held_ms: the knowledge delay) and live (held by channel_open_ms: not "
             "a backlog the channel's first HAVEs announce).",
}
STANDARD_KEYS = {"seconds": "k, sec", "viewer_seconds": "k, viewer, sec", "viewers": "k, viewer",
                 "events": "k, t_us, kind, node", "bins": "k, bin", "node_bins": "k, bin, node",
                 "reads": "k, viewer, t_ms", "forecast": "k, node, peer, seq",
                 "stalls": "k, viewer, delay_ms, t_ms", "parents": "k, node, stream, t_ms",
                 "asks": "k, node, ask, peer", "haves": "k, node, peer, seq",
                 "arrivals": "k, node, seq, incarnation", "spec_requests": "k, node, peer, seq, t_ms",
                 "parks": "k, node, peer, seq, t_ms"}
STANDARD_JOINS = {"seconds": "jobs USING (k, src); verdict_seconds USING (k, src, sec)",
                  "bins": "jobs USING (k, src) (bin_ms, the bins' width); node_bins USING (k, src, bin) (each "
                          "node's in the bin); seconds and verdict_seconds ON (k, src) and sec = t_ms // 1000 (the "
                          "second the bin starts in)",
                  "arrivals": "viewers USING (k, src, node); asks and haves ON (k, src) and arrivals.node = their "
                              "peer, seq, arrivals.incarnation = their peer_incarnation (the peer's holds: the "
                              "earliest is their views' truth); spec_requests ON (k, src) and arrivals.node = "
                              "spec_requests.peer, seq, arrivals.t_ms = held_ms, and parks ON (k, src, node, seq) "
                              "and arrivals.t_ms = parks.held_ms (the holder's hold)",
                  "spec_requests": "viewers USING (k, src, node) (the requester); asks USING (k, src, t_ms, node, "
                                   "peer, seq) for round 'spec' AND picked (the decision; its rivals by ask); "
                                   "parks ON (k, src) and parks.node = peer, parks.peer = node, seq, "
                                   "parks.t_ms between t_ms and t_outcome_ms (the holder's park or refusal); "
                                   "arrivals ON (k, src) and arrivals.node = peer, seq, arrivals.t_ms = held_ms (the "
                                   "holder's hold)",
                  "parks": "viewers USING (k, src, node) (the holder); spec_requests ON (k, src) and "
                           "spec_requests.node = parks.peer, spec_requests.peer = parks.node, seq, parks.t_ms "
                           "between their t_ms and t_outcome_ms (the request as its requester logged it), asks the "
                           "same with picked (either round's); arrivals ON (k, src, node, seq) and arrivals.t_ms = "
                           "parks.held_ms (the holder's hold)",
                  "viewer_seconds": "viewers USING (k, src, viewer)", "reads": "viewers USING (k, src, viewer)",
                  "events": "viewers USING (k, src, node)", "node_bins": "viewers USING (k, src, node)",
                  "forecast": "viewers USING (k, src, node)", "stalls": "viewers USING (k, src, viewer)",
                  "parents": "viewers USING (k, src, node)", "viewers": "reads, viewer_seconds, stalls USING (k, src, viewer)",
                  "asks": "viewers USING (k, src, node); spec_requests USING (k, src, t_ms, node, peer, seq) for "
                          "round 'spec' AND picked; arrivals ON (k, src) and arrivals.node = asks.peer, seq, "
                          "arrivals.incarnation = asks.peer_incarnation (the peer's holds, the view's truth)",
                  "haves": "viewers USING (k, src, node); arrivals ON (k, src) and arrivals.node = haves.peer, seq, "
                           "arrivals.incarnation = haves.peer_incarnation (the peer's holds, the view's truth); "
                           "asks ON (k, src, node, peer, seq) for a request's knowledge"}
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
    "startup_s": "the seconds a collapse must last past: those the run's startup holds from the stream's start "
                 "(STARTUP_SECS, fewer when another phase window takes over sooner)",
    "viewer": "the viewer (viewers.viewer); in phase_windows a join's, whose own startup it is, NULL for the run's",
    "node": "the node of the window's event (events.node, strims-sim's host); NULL for the start's",
    "anchor_s": "when the window's event came, s from the stream's start",
    "end_s": "when a span's until event came, s from the stream's start; NULL for a step, a level or no end",
    "stop_s": "when the window stops holding seconds: its anchor (a span's end, or the run's end) plus horizon_s",
    "horizon_s": "how long the window lasts past its anchor, or past its span's end (the vocabulary's horizon_s)",
    "phase": f"{either(verdict_phases())}: the vocabulary's phases, as the table's description gives them",
    "healthy": "no trigger held this second", "trees_bad": "the smoothed hit share under TREES_SHARE",
    "deliv_bad": "the delivery share under DELIVERY_SHARE", "lag_high": "the lag q0.9 past LAG_HIGH_MS",
    "lag_rise": "the lag rising past LAG_SLOPE ms/s over LAG_SLOPE_SECS while past LAG_SLOPE_FLOOR_MS",
    "hit_share": "the honest viewers' hits over what they needed",
    "lag_q": "the honest present viewers' nearest-rank q0.9 lag, ms",
    "verdict_version": "the verdicts' version (simdb.derived_version)",
}
# event_kinds' columns from the vocabulary, as kind_docs gives them.
KIND_COLS = {"kind": "VARCHAR", "sim": "VARCHAR", "class": "VARCHAR", "scope": "VARCHAR",
             "description": "VARCHAR", "value_meaning": "VARCHAR", "node": "VARCHAR", "peer": "VARCHAR",
             "detail": "VARCHAR", "details": "VARCHAR[]", "notes": "VARCHAR", "env_gate": "VARCHAR",
             "replaced_by": "VARCHAR"}
# The sim event_kinds names derived_events' kinds by.
DERIVED_SIM = "derived"


def kind_docs():
    """event_kinds' records of the vocabulary's kinds, retired ones too: a
    kind's for each sim whose runs may hold it (vocab.holders), with that
    sim's note and replacement; a derived kind's for DERIVED_SIM."""
    out = []
    for kid in [*vocab.kinds(), *vocab.retired()]:
        k = vocab.kind(kid)
        for sim in [DERIVED_SIM] if k["class"] == "derived" else vocab.holders(kid):
            out.append({"kind": kid, "sim": sim, "class": k["class"], "scope": vocab.scope(kid),
                        "description": k["description"],
                        "value_meaning": vocab.value_text(k), "node": k.get("node"), "peer": k.get("peer"),
                        "detail": k.get("detail"), "details": k.get("details"), "notes": vocab.notes(kid, sim),
                        "env_gate": k.get("env_gate"),
                        "replaced_by": vocab.current(kid, sim) if "replaced_by" in k else None})
    return out


def catalogs_sql(cols, tmp, recount=True):
    """metric_catalog, event_kinds and table_catalog, made again: the
    events of each kind counted again, or without `recount` kept as
    event_kinds holds them (the runs unchanged)."""
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
    for t, view in [*names.items(), *((v, v) for v in VERDICT_TABLES)]:
        for c in cols.get(t, {}):
            if c in ("k", "src"):
                continue
            by = col_docs.get((t, c), {})
            desc = (rdocs.get(c) if t == "rows" else
                    (VERDICT_DOCS.get(c) or mdocs.get(c) or mdocs.get(c.replace("_censored", "")))
                    if t in VERDICT_TABLES else
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
    # event_kinds: the vocabulary's kinds, and the kinds the runs hold that
    # it lacks (undocumented warns of them)
    seen = []
    if not recount:
        seen.append("SELECT sim, kind, n, first_seen, last_seen FROM main.event_kinds WHERE n > 0")
    elif "events" in cols:
        seen.append("SELECT j.sim, e.kind, count(*) AS n, min(j.ran_at) AS first_seen, max(j.ran_at) AS last_seen "
                    "FROM raw.events e JOIN main.jobs j USING (k, src) GROUP BY ALL")
    if recount and "derived_events" in cols:
        seen.append(f"SELECT {q(DERIVED_SIM)} AS sim, e.kind, count(*) AS n, min(j.ran_at), max(j.ran_at) "
                    "FROM main.derived_events e JOIN main.jobs j USING (k) GROUP BY ALL")
    seen_sql = " UNION ALL ".join(seen) or ("SELECT NULL::VARCHAR AS sim, NULL::VARCHAR AS kind, 0::BIGINT AS n, "
                                            "NULL::TIMESTAMPTZ AS first_seen, NULL::TIMESTAMPTZ AS last_seen WHERE false")
    doc_cols = ", ".join(f"d.{ident(c)}" for c in KIND_COLS if c not in ("kind", "sim"))
    out.append(f"""CREATE OR REPLACE TABLE main.event_kinds AS
WITH seen AS ({seen_sql}),
doc AS (SELECT * FROM {read_jsonl(jsonl(tmp / 'event_kinds.jsonl', kind_docs()), KIND_COLS)})
SELECT coalesce(d.kind, s.kind) AS kind, coalesce(d.sim, s.sim) AS sim, {doc_cols},
       d.kind IS NOT NULL AS documented, coalesce(s.n, 0) AS n, s.first_seen, s.last_seen
FROM doc d FULL JOIN seen s ON s.kind = d.kind AND s.sim = d.sim ORDER BY sim, kind;""")
    return out


def undocumented():
    """The kinds event_kinds holds that the vocabulary doesn't give the
    sims whose events hold them, {kind: [sim]}."""
    out = {}
    for r in lake("SELECT kind, sim FROM event_kinds WHERE NOT documented ORDER BY kind, sim;"):
        out.setdefault(r["kind"], []).append(r["sim"])
    return out


def own_docs(text):
    """The columns a doc table's columns cell documents in their own
    words, {name: (form, text)}: each backticked name outside every
    parenthesis with its meaning right after it, in parentheses (form
    "paren") or after a colon to the cell's end ("colon"), as
    column_docs reads it. A name inside another's parentheses is a value
    or a cross-reference there (`reason`'s `silent`), no column's doc,
    though column_docs takes its text for the column of that name."""
    out, depth, at = {}, 0, 0
    for m in BACKTICK.finditer(text):
        between = text[at:m.start()]
        depth, at = depth + between.count("(") - between.count(")"), m.start()
        name = m.group(1)
        if depth or name in out:
            continue
        p = paren(text, m.end() + 1) if text[m.end():m.end() + 1] == " " else None
        if p:
            out[name] = ("paren", p)
        elif text[m.end():m.end() + 2] == ": " and text[m.end() + 2:].strip():
            out[name] = ("colon", text[m.end() + 2:].strip())
    return out


# What a column's doc is when it isn't its own (doc_gaps), in the order
# warn_undocumented gives them.
GAPS = {"none": "no doc", "sentence": "a doc sentence's, not the column's own",
        "nested": "text inside another column's parentheses",
        "runs on": "a colon's doc running on into other columns' docs"}


def doc_gaps(docs, tables, verdict_docs):
    """The columns among `tables` ({view: (the sims' table it is or None,
    {column: its description in metric_catalog})}) whose doc isn't their
    own, {view: {gap: [column]}} (GAPS). A sims' table's own docs are
    its doc tables' (`docs`, doc_tables'), own_docs' of either sim's
    cell; a verdict table's are `verdict_docs`' names. A sims' table no
    doc table documents is left out: the log of a mechanism only a
    branch has keeps its docs in that branch's sims."""
    out = {}
    for view, (t, cols) in sorted(tables.items()):
        if t is None:
            own, nested, cells = verdict_docs, set(), []
        else:
            cells = [docs[s][t][1] for s in sorted(docs) if t in docs[s]]
            if not cells:
                continue
            own = {c: d for cell in cells for c, d in own_docs(cell).items()}
            nested = {c for cell in cells for c in column_docs(cell)}
        gaps = {}
        for c, desc in sorted(cols.items()):
            if c in own:
                form, text = own[c] if t is not None else ("paren", "")
                gap = "runs on" if form == "colon" and set(BACKTICK.findall(text)) - {c} & set(cols) else None
            else:
                gap = "none" if desc is None else "nested" if c in nested else "sentence"
            if gap:
                gaps.setdefault(gap, []).append(c)
        if gaps:
            out[view] = {g: gaps[g] for g in GAPS if g in gaps}
    return out


def column_gaps():
    """doc_gaps over the catalog: the columns of the sims' tables and the
    verdict tables, as metric_catalog holds them, against the docs this
    process reads."""
    views = {v: t for t, v in sim_tables(lake_columns()).items()}
    tables = {}
    for r in lake("SELECT tbl, name, description FROM metric_catalog WHERE tbl NOT IN ('rows', 'suite');"):
        t = views.get(r["tbl"])
        if t is not None or r["tbl"] in VERDICT_TABLES:
            tables.setdefault(r["tbl"], (t, {}))[1][r["name"]] = r["description"]
    # A verdict column's own doc, as catalogs_sql takes it.
    metrics = run_metric_docs()
    verdict_docs = {c: None for t, cols in tables.values() if t is None for c in cols
                    if c in VERDICT_DOCS or c in metrics or c.replace("_censored", "") in metrics}
    return doc_gaps(doc_tables(), tables, verdict_docs)


def names_text(names, most=8):
    return ", ".join(names[:most]) + (f" and {len(names) - most} more" if len(names) > most else "")


def warn_undocumented():
    """Warns on stderr of each kind of undocumented's, a kind the sims
    write that the vocabulary should name; and of each table's columns
    column_gaps finds, whose doc isn't their own."""
    for kind, sims in undocumented().items():
        print(f"index: event kind {kind} ({', '.join(sims)}) isn't in the vocabulary, "
              f"{vocab.DOCUMENT.relative_to(ROOT)}", file=sys.stderr)
    where = " and ".join(str(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p) for p in DOC_FILES[:2])
    for view, gaps in column_gaps().items():
        source = "catalog.VERDICT_DOCS or verdict.run_metrics' docstring" if view in VERDICT_TABLES else where
        print(f"index: {view}'s columns without a doc of their own ({source}): "
              + "; ".join(f"{GAPS[g]} for {names_text(cs)}" for g, cs in gaps.items()), file=sys.stderr)


# --- scoring ----------------------------------------------------------------------

SCORING = Path(__file__).with_name("sql") / "scoring.sql"
# A metric's SQL reads a job's row (r), verdict (v) and job (j).
FIELD = re.compile(r"\b([rvj])\.([a-z_][a-z0-9_]*)\b")
CONST = re.compile(r"@\{([a-z_][a-z0-9_]*)\}")
ROW_FIELD = re.compile(r"@\{r\.([a-z0-9_]+)\}")
MACRO = re.compile(r"^CREATE OR REPLACE (MACRO|TABLE|VIEW) ([a-z_0-9]+)\s*(\([^)]*\))?", re.M)
TEMP = re.compile(r"^CREATE OR REPLACE (MACRO|TABLE|VIEW) ", re.M)
# A scoring.sql statement's comment line (`-- needs: ...`) naming the
# tables (table, or table.column) it reads that a catalog indexed by older
# code may lack.
NEEDS = re.compile(r"needs: (.+)")


def comment_above(text, at):
    """The text of the `--` lines right above offset `at` of `text`."""
    before = text[:at].rstrip("\n").splitlines()
    out = []
    while before and before[-1].startswith("--"):
        out.insert(0, before.pop()[2:].strip())
    return out


def needed(text, have):
    """scoring.sql's `text` with each macro whose `-- needs:` names a table
    or column that `have` ({table: its columns}) lacks made one that fails
    saying so when called, so the others still score over a catalog its
    index made with older code; `text` as it is for `have` None (every
    one there)."""
    if have is None:
        return text

    def lacks(need):
        table, _, col = need.partition(".")
        return table not in have or bool(col) and col not in have[table]

    out, at = [], 0
    for m in MACRO.finditer(text):
        needs = [n for line in comment_above(text, m.start()) if (x := NEEDS.match(line)) for n in x.group(1).split()]
        missing = [n for n in needs if lacks(n)]
        if not missing or m.group(1) != "MACRO":
            continue
        end = text.find(";", m.end())
        why = q(f"{m.group(2)} reads {', '.join(missing)}, which this catalog lacks: index it with this checkout")
        body = f"TABLE SELECT error({why}) AS error" if " AS TABLE" in text[m.end():m.end() + 20] else f"error({why})"
        out.append(f"{text[at:m.start()]}CREATE OR REPLACE MACRO {m.group(2)}{m.group(3) or '()'} AS {body}")
        at = end
    return "".join(out) + text[at:]


def scoring_consts(suite=None):
    """scoring.sql's @{name} constants: suite.py's and analyze.py's (the
    vocabulary's phases among them, the verdict's), and t975_at's
    expansion (t975_expansion)."""
    s, a = suite or suite_module(), verdict
    z, g = t975_expansion()
    return {"guard_ratio": s.GUARD_RATIO, "lag_ceiling": a.DELIVERY_TARGET_MS,
            "delay_target_ms": a.DELIVERY_TARGET_MS, "delay_max_ms": a.DELIVERY_MAX_MS,
            "t975_df": T975_DF, "t975_z": double(z),
            # The vocabulary's phases, in its order (dynamics_phases'), and
            # the one past the verdict's seconds (viewer_phases_of's).
            "phases": "[" + ", ".join(q(p) for p in a.VOCAB["phases"]) + "]",
            "tail": q(query.TAIL),
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
    verdicts (v) and job_runs (j), so a field they lack reads NULL, and
    the database's tables' (have: None for one holding every one a macro
    needs), so a macro needing one it lacks fails saying so (needed). With
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
    out.append(needed(text, cols.get("have")))
    sql = "\n".join(out)
    return TEMP.sub(lambda m: f"CREATE OR REPLACE TEMP {m.group(1)} ", sql) if temp else sql


def scoring_statements(names, have):
    """scoring.sql's statements making the objects `names`, their @{...}
    constants filled in, each one whose `-- needs:` tables are all in
    `have` (a set of table names): the macros simlab.query's views over
    the cache take from it, so a definition both read is one."""
    consts = scoring_consts()
    text = CONST.sub(lambda m: str(consts[m.group(1)]), SCORING.read_text())
    out = []
    for m in MACRO.finditer(text):
        needs = {n for line in comment_above(text, m.start()) if (x := NEEDS.match(line)) for n in x.group(1).split()}
        if m.group(2) in names and needs <= set(have):
            out.append(text[m.start():text.find(";", m.end()) + 1])
    return out


def scoring_version(suite=None):
    """A hash of what scoring_sql makes: scoring.sql, METRICS and the
    rest of suite.py's it reads, the constants and scoring_sql itself."""
    s = suite or suite_module()
    parts = [SCORING.read_text(), repr([s.METRICS, s.GUARDRAILS, s.HOST_METRICS, s.SHOWN, scoring_consts(s),
                                        t975()]), *SCORING_SOURCES]
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()[:12]


# scoring_sql's and getter_sql's sources, which scoring_version hashes: read
# as the module loads, so a process hashes the code it runs, as
# verdict.DERIVE_SOURCES is.
SCORING_SOURCES = (inspect.getsource(scoring_sql), inspect.getsource(getter_sql))


def scoring_cols(columns, have=None):
    """scoring_sql's `cols` from `columns`, {table: its columns}, and
    `have`, the database's tables (needed's: None for every one)."""
    return {"r": set(columns.get("rows", ())), "v": set(columns.get("verdicts", ())), "j": None, "have": have}


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
        comment = [c for c in comment_above(text, m.start()) if not NEEDS.match(c)]
        body = text[m.end():text.find(";", m.end())]
        kind = ("table macro" if " AS TABLE" in body[:200] else "macro") if kind == "MACRO" else kind.lower()
        desc = " ".join(c for c in comment if c) or None
        recs.append({"name": name, "kind": kind, "grain": None, "keys": params and re.sub(r"\s+", " ", params),
                     "joins": None,
                     "description": desc})
    recs.append({"name": "scoring.sql", "kind": "macros", "grain": None, "keys": None, "joins": None,
                 "description": " ".join(h for h in header if h)})
    return recs


# --- the docs' version ------------------------------------------------------------


def docs_version():
    """A hash of what the doc tables (catalogs_sql's) are made from but the
    catalog's columns, which change only with an index that makes them
    again anyway: what the catalog reads of the sims' docs (DOC_FILES: their
    doc tables, sentences and row comments), of suite.py's (its module
    docs' sentences, METRICS with their comments, GUARDRAILS), of
    run_metrics' docstring, of scoring.sql (scoring_docs) and of the
    vocabulary (kind_docs), each as DOCS_VALUES gives it now; the registries
    (DOCS_DATA), as this process holds them; and the code that puts them
    together with the columns (DOCS_CODE), as it loaded."""
    parts = [canonical(f()) for f in DOCS_VALUES] + [canonical(globals()[n]) for n in DOCS_DATA]
    return hashlib.sha256("\n".join([repr(parts), *DOCS_SOURCES]).encode()).hexdigest()[:12]


def lake_docs_version():
    """docs_version; none, with a warning, when what it reads doesn't
    load."""
    try:
        return docs_version()
    except Exception as e:
        print(f"index: no docs_version: what the docs are made from doesn't load ({e})", file=sys.stderr)
        return None


def canonical(v):
    """`v` for repr to give the same text in every process: its sets
    sorted, its patterns their text."""
    if isinstance(v, re.Pattern):
        return v.pattern
    if isinstance(v, (set, frozenset)):
        return sorted((canonical(x) for x in v), key=repr)
    if isinstance(v, dict):
        return {k: canonical(x) for k, x in v.items()}
    if isinstance(v, (list, tuple)):
        return [canonical(x) for x in v]
    return v


# What the doc tables are made from, which docs_version hashes: the docs as
# the catalog reads them, each one's function; the registries, by name;
# and the code that puts them together with the columns, its source read
# as the module loads, so a process hashes the code it runs (as
# SCORING_SOURCES are). catalogs_sql reads nothing else but the columns and
# the SQL helpers (q, ident, jsonl, read_jsonl).
DOCS_VALUES = (doc_tables, doc_sentences, row_comments, run_metric_docs, suite_metrics, guards, scoring_docs, kind_docs)
DOCS_DATA = ("TABLE_DOCS", "NORMALIZED", "STANDARD_KEYS", "STANDARD_JOINS", "VERDICT_DOCS", "VERDICT_TABLES",
             "KIND_COLS", "DERIVED_SIM", "UNITS", "PLAIN", "BACKTICK", "OWN", "META")
DOCS_CODE = (catalogs_sql, sim_tables, row_docs, column_docs, paren, unit_of)
DOCS_SOURCES = tuple(inspect.getsource(f) for f in DOCS_CODE)


# --- whose docs ---------------------------------------------------------------------


def docs_of(info):
    """Whose docs an index over a catalog whose catalog_info is `info`
    makes: {"version": this process's docs_version (None where it doesn't
    load), "main": main's docs key (main_docs'; None where there is no main
    to keep the docs for), "own": whether this process's docs are main's,
    "moved": whether it makes the doc tables again with nothing else to
    do}. The shared catalog's docs are main's: a process whose doc files
    (docs_ids) are main's has main's docs; so has one whose docs_version is
    that of the doc tables made by such a process at main's docs key now
    (catalog_info's docs_version and docs_main). Any other (a branch's
    worktree, a process whose code is older than main's: the console
    server's) leaves the doc tables as they are; an index of its that
    changes the data marks them behind it (docs_behind), and the next
    index whose docs are main's makes them again, counting again."""
    dv = lake_docs_version()
    main = main_docs()
    km = None if main is None else docs_key(main)
    own = (main is None or docs_key(docs_ids()) == km
           or bool(dv) and info.get("docs_version") == dv and info.get("docs_main") == km)
    moved = bool(dv) and own and (info.get("docs_version") != dv or bool(info.get("docs_behind"))
                                  or km is not None and info.get("docs_main") != km)
    return {"version": dv, "main": km, "own": own, "moved": moved}


def docs_kept(docs):
    """The line an index whose docs aren't main's prints."""
    return f"index: the doc tables left as main's (this checkout's docs, docs_version {docs['version']}, aren't main's)"


def docs_ids():
    """The files the doc tables are made from, as this process has them,
    {path from the checkout's root: git blob id}: simlab's modules and the
    vocabulary as they loaded (LOADED_DOCS), the sims' docs (DOC_FILES),
    suite.py's and simlab's SQL as an index reads them."""
    out = dict(LOADED_DOCS)
    for p in (*DOC_FILES, SUITE_PY, *sorted(SCORING.parent.glob("*.sql"))):
        with contextlib.suppress(OSError):
            out[relative(p)] = blob_id(Path(p).read_bytes())
    return out


def main_docs():
    """The files docs_ids names as main holds them, {path: blob id}, from
    git; None for a lake other than the main checkout's (a fixture's, or
    one SIM_DB_ROOT puts elsewhere: its docs are its indexer's), or where
    git can't say (no repository, no main, or a main without simlab)."""
    try:
        common = Path(store_host.git("rev-parse", "--path-format=absolute", "--git-common-dir", cwd=ROOT)).parent
        if lake_root().resolve() != (common / "target" / "sim-db" / "lake").resolve():
            return None
        listed = store_host.git("ls-tree", "main", "--", "hack/simlab/", "hack/simlab/sql/",
                                *(relative(p) for p in (*DOC_FILES, SUITE_PY, vocab.DOCUMENT)), cwd=ROOT)
    except (subprocess.CalledProcessError, OSError):
        return None
    out = {}
    for line in listed.splitlines():
        meta, _, path = line.partition("\t")
        if meta.split()[1:2] == ["blob"] and docs_path(path):
            out[path] = meta.split()[2]
    return out if "hack/simlab/catalog.py" in out else None


def docs_path(path):
    """Whether `path` (from the checkout's root) is one of docs_ids'."""
    return (bool(re.fullmatch(r"hack/simlab/[^/]+\.py|hack/simlab/sql/[^/]+\.sql", path))
            or path in {relative(p) for p in (*DOC_FILES, SUITE_PY, vocab.DOCUMENT)})


def docs_key(ids):
    """docs_ids' or main_docs' {path: blob id}, hashed."""
    return hashlib.sha256(json.dumps(sorted(ids.items())).encode()).hexdigest()[:12]


def relative(p):
    """Path `p` from the checkout's root, as git names it."""
    with contextlib.suppress(ValueError):
        return Path(p).resolve().relative_to(ROOT).as_posix()
    return str(p)


def blob_id(data):
    """The git blob id of bytes `data`."""
    return hashlib.sha1(b"blob %d\0" % len(data) + data).hexdigest()


# simlab's modules and the vocabulary, as this process loaded them (read as
# this module loads, as DOCS_SOURCES are, which in a process that indexes
# is with the modules it runs): a process that outlives an edit to them
# names the code it runs.
LOADED_DOCS = {relative(p): blob_id(p.read_bytes())
               for p in (*sorted(Path(__file__).parent.glob("*.py")), vocab.DOCUMENT) if p.exists()}


# --- maintenance ------------------------------------------------------------------


def compact(keep_minutes=15):
    """Merges the written tables' small files, and lets go of the snapshots
    the next one superseded more than `keep_minutes` ago (and the files
    only they read): a reader opened on a snapshot while it was the latest
    has that long after the next index before its catalog goes (the latest
    stays whatever its age)."""
    with latest():
        have = set(lake_columns())
        gone = [r["id"] for r in lake(
            "SELECT snapshot_id AS id FROM (SELECT snapshot_id, lead(snapshot_time) OVER (ORDER BY snapshot_id) AS "
            "next FROM ducklake_snapshots('lake')) "
            f"WHERE next < now() - INTERVAL {int(keep_minutes)} MINUTE ORDER BY id;")]
        sql = [f"CALL ducklake_merge_adjacent_files('lake', {q(t)}, schema => 'raw');" for t in WRITTEN if t in have]
        if gone:
            sql.append(f"CALL ducklake_expire_snapshots('lake', versions => [{', '.join(str(int(v)) for v in gone)}]);")
        sql.append(f"CALL ducklake_cleanup_old_files('lake', older_than => now() - INTERVAL {int(keep_minutes)} MINUTE);")
        lake("\n".join(sql), write=True, out=None)
        null_stats()


def skipped_path():
    """lake/compact.skipped: why the last index to compact skipped it (the
    module docs); there until an index compacts."""
    return lake_root() / "compact.skipped"


def compacted(say):
    """compact(), retried while the catalog's lock refuses it (each of its
    steps rolls back whole, and the merges again find what is left): None
    once done, or why it was skipped, the lock refusing it past the retries
    (skipped_path keeps it, and the next index compacts)."""
    try:
        retried(compact, say, "compacting")
    except store_host.DuckError as e:
        if not locked_out(e):
            raise
        why = f"the catalog stayed locked through {len(WRITE_RETRY_S) + 1} tries: " + " ".join(str(e).split())
        skipped_path().write_text(why + "\n")
        say("write", f"index: compacting skipped ({why}); the next index compacts")
        return why
    skipped_path().unlink(missing_ok=True)
    return None


def owed(say):
    """The compacting an earlier index skipped (skipped_path), done by an
    index with nothing else to do: compacted()'s, or None with none owed."""
    if not skipped_path().exists():
        return None
    say("write", "index: compacting (the last index skipped it)")
    return compacted(say)


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


def fetch(exp, cells=None, arms=None, tables=(), workers=None, limit_mb=None, progress=None):
    """Copies what S3 holds of experiment `exp`'s jobs (`cells`, `arms`) into
    the cache, then indexes (`workers`, `progress` index's): the flushes
    holding their tables (store.pull of each farm run the cache lacks one
    of), and with `tables` (of SAMPLE_TABLES) the fewest flushes giving the
    jobs those (store's sample_plan and pull_samples, which refuses past
    `limit_mb`). Returns what it pulled and what S3 charged for it."""
    say = sayer(progress)
    if not catalog_path().exists():
        index(workers, progress)
    bad = [t for t in tables if t not in store_host.SAMPLE_TABLES]
    if bad:
        sys.exit(f"--tables {', '.join(bad)}: fetch takes the sample tables ({', '.join(store_host.SAMPLE_TABLES)}); "
                 "the others come with a flush's sim-data")
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
    out = Fetched(exp)
    with scratch() as tmp:
        jsonl(tmp / "keys.jsonl", [{"k": k} for k in keys])
        holders = lake(f"SELECT DISTINCT f.src FROM raw.farm_index f SEMI JOIN {read_jsonl(tmp / 'keys.jsonl', {'k': 'VARCHAR'})} w "
                       "USING (k) WHERE f.src LIKE 'farm/%';")
        lack = [r["src"] for r in holders if not store.have_src(r["src"])]
        if lack:
            say("pull", f"{exp}: pulling the farm runs of {len(lack)} flushes the cache lacks", 0, len(lack))
            pulls, out.gone = store.pull_sources(lack)
            out.pulled = sorted({s for p in pulls for s in p.pulled})
            if out.gone:
                say("pull", f"{len(out.gone)} of them are gone from S3: their jobs' tables are left out")
        if tables:
            srcs = sorted({s for s in [r["src"] for r in holders] if store.have_src(s)} |
                          {r["src"] for r in lake(f"SELECT DISTINCT src FROM raw.runs SEMI JOIN "
                                                  f"{read_jsonl(tmp / 'keys.jsonl', {'k': 'VARCHAR'})} w USING (k);")
                           if store.have_src(r["src"])})
            v = (f"CREATE VIEW runs_all AS SELECT k, src FROM read_parquet({verdict.plist(verdict.files(srcs, 'runs'))}, "
                 f"union_by_name=true);\nCREATE VIEW runs AS SELECT k FROM "
                 f"{read_jsonl(tmp / 'keys.jsonl', {'k': 'VARCHAR'})};")
            store.pull_samples(v, tuple(tables), limit_mb=limit_mb, say=lambda m: say("pull", m))
    index(workers, progress)
    out.s3_usd, out.ledger_lines = s3_spent(before)
    return out


# --- the readers ----------------------------------------------------------------------


def pool(name, size=1, guard=engine.Guard()):
    """An engine.Pool of `size` guarded readers of the catalog, `guard`'s
    limits theirs, spilling under the cache's tmp/<name>/; the extensions
    installed and checked first (extensions)."""
    extensions()
    return engine.Pool(lake_root(), store_host.cache_root() / "tmp" / name, size=size, guard=guard)


# --- the console's schema ------------------------------------------------------------

# What GET /console/schema answers, a query each, and how many rows each
# may hold.
SCHEMA = {
    "tables": "SELECT table_schema AS schema, table_name AS name, "
              "list({name: column_name, type: data_type} ORDER BY ordinal_position) AS columns "
              "FROM information_schema.columns WHERE table_catalog = 'lake' GROUP BY ALL ORDER BY ALL",
    "table_catalog": "SELECT * FROM table_catalog ORDER BY name",
    "metric_catalog": "SELECT * EXCLUDE (first_seen, last_seen) FROM metric_catalog ORDER BY tbl, name",
    "event_kinds": "SELECT * FROM event_kinds ORDER BY sim, kind",
    "experiments": "SELECT exp_id, kind, started_at, tiers, arm_labels, base, question, cells, jobs "
                   "FROM experiments ORDER BY started_at DESC NULLS LAST, exp_id",
    "catalog_info": "SELECT * FROM catalog_info",
}
SCHEMA_ROWS = 200_000


def schema(pool):
    """{part: [row]} of SCHEMA's queries over `pool` (an engine.Pool of the
    catalog's guarded readers), each row a dict of the console's JSON
    values (engine.Result.json)."""
    out = {}
    for part, sql in SCHEMA.items():
        j = pool.query(sql, max_rows=SCHEMA_ROWS, timeout_s=60).json()
        names = [c["name"] for c in j["columns"]]
        out[part] = [dict(zip(names, r)) for r in j["rows"]]
    return out
