#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' tables: a store of runs' data, and views over them in DuckDB.

  hack/sim-db/simdb.py duckdb
  hack/sim-db/simdb.py fold DB JOB_DIR... [--src SRC]
  hack/sim-db/simdb.py export DB DEST
  hack/sim-db/simdb.py pull --run RUN [--reads [--cell C] [--arm A] [--seed N]] [--dry-run]
  hack/sim-db/simdb.py gc [--days N]
  hack/sim-db/simdb.py sql QUERY [selection] [--reads] [--json]
  hack/sim-db/simdb.py pooled [selection] [--by none|phase|bin|viewer] [--bin MS] [--json]
  hack/sim-db/simdb.py aligned EVENT [selection] [--field F] [--bin MS] [--per run|node] [--node N]
                                     [--detail GLOB] [--pool] [--before S] [--after S] [--nth N] [--json]
  hack/sim-db/simdb.py events [selection] [--json]
  hack/sim-db/simdb.py osc [selection] [--json]
  hack/sim-db/simdb.py rows [selection] [--fields A,B,...]

A run of either sim given an output dir (`SIM_OUT`, `STRIMS_SIM_OUT`)
writes its tables there as CSV files, each beside a `<table>.schema.json`
of its columns' names and types, and a `series_meta.json` (the sims'
rustdoc: crates/strims-sim/src/tables.rs, and the ppspp sim's
`Sim::write_tables`):

  seconds         the run second by second, as the row's `series` has it
  viewer_seconds  each viewer's p99 lag and reads each second (strims-sim
                  writes none for a second its viewer wasn't there in)
  reads           every read: viewer, t_ms (true time from the stream's
                  start), seq, lag_ms
  stalls          each viewer's stalls, playing its reads at delays of
                  1250 and 4250 ms (analyze.py's DELIVERY_TARGET_MS and
                  DELIVERY_MAX_MS): viewer, delay_ms, t_ms (when the stall
                  began), stall_ms; macros.sql's playback(delay_ms) plays
                  `reads` the same way, at any delay
  bins            finer bins (SIM_BIN_MS / STRIMS_SIM_BIN_MS wide, 100 ms
                  by default), one wide row a bin: t_ms, then counters
                  summed over it and gauges at its end: the series'
                  fields, the uplinks' bytes and packets, and per viewers
                  (`viewer_`) and seed (`seed_`) timeouts, wasted chunks,
                  answers refused (a full queue) and dropped (at write
                  time), parent switches, sheds taken and sent,
                  message bytes by kind, payloads and HAVE-only ones,
                  retransmits, drops and push and answer waits (strims-sim
                  also the window model's window cuts and timeouts, and
                  the acknowledgements alone it sends, packets and bytes
                  the uplinks' leave out: TCP's pure ACKs
                  (*_up_ack_packets, *_up_ack_bytes) and the data
                  channels' SACKs (*_up_sack_packets, *_up_sack_bytes));
                  -1 where a run has no reading
  node_bins       with SIM_BIN_NODES=1 / STRIMS_SIM_BIN_NODES=1, each
                  node's bins
  viewers         the viewers, numbered as the others number them, with
                  hostile and churned, and when each arrived and left for
                  good (arrive_ms, negative for one there from the start,
                  and depart_ms), which the verdicts' series carry
  events          the sims' timed events: t_us, kind, node, peer, value,
                  detail
  parents         every change of a node's push parent of a stream: t_ms,
                  node, stream, parent (the node from then on, -1 for
                  none), reason (open, switch, prune, shed, lost), nodes
                  numbered as in events
  forecast        with SIM_FORECAST=1 / STRIMS_SIM_FORECAST=1 (large: a
                  row per neighbour per chunk announced), every sample of
                  a neighbour's announcement lag, with the estimate it
                  updated: t_ms, node, viewer, peer, stream,
                  seq, sample_ms, mean_ms, horizon_ms, samples, fresh,
                  lazy, first_sight, run, seen_ms, held_ms (when the peer
                  first held the chunk: -1 where never), live (held after
                  both nodes took their places)

A run's `bin_ms` is its bins' width: `--bin` over a bins column is a
multiple of every selected run's. The CLI's cache root is `SIM_DB_ROOT`
when set.

`fold` adds jobs' tables to a local DuckDB file, one transaction a batch,
each job's rows under its key `k` and the batch's `src`, beside two of
its own: `runs` (one a job: its cell, arm, seed, env, the row as JSON and
the meta) and `scalars` (the row's numbers, long form, for SQL). Columns
come from the schema files, so a table gains the columns a later sim
writes (`ADD COLUMN IF NOT EXISTS`); folding a job again replaces it.
`export` writes a store to zstd Parquet, a file a table, with a
`manifest.json`. suite.py folds its local runs this way and exports each
invocation's store into the cache, `<main checkout>/target/sim-db/cache/`
(suite/<worktree>/<invocation>/), which every worktree shares; the farm
puts its instances' flushes in S3, and `pull` copies a run's into the
cache (farm/<run>/...). Its reads and forecast logs (the bulky tables,
sim-samples/) come with `--reads`: the whole run's, or with `--cell`,
`--arm` and `--seed` only the flushes holding those jobs (the manifests'
keys; a job held twice, a verify copy, from the fewest flushes), and the
queries that read them (pooled, aligned on a read lag, forecast, and sql
with `--reads`) pull their selection's that way when the cache lacks
them, noting the size, at most `--pull-mb`. `--dry-run` lists what a pull
would fetch, asking S3 nothing. What a pull costs S3 goes to the farm's
S3 ledger (pull). Nothing in the cache is kept for long: a run is
reproduced when it's wanted again, and `gc` deletes what's older than
`--days` (7).

The queries read the cache's tables and, made from them and kept under
derived/<table>/v=<derived_version>/ (a hash of analyze.py's
VERDICT_VERSION and of the derived tables' columns and code, so a change
to either makes them again), the verdicts' tables: `verdict_seconds`
(each second's phase, health and what judged it), `verdicts` (a run's
verdict, the seconds a collapse must last past, `startup_s`
(analyze.startup), and analyze.run_metrics), `episodes` (healthy,
unhealthy and collapse spells) and `derived_events` (start, seed_down,
seed_up, present_join, present_leave, onset, recovered, collapse:
DERIVED_KINDS, none of them a kind the sims' events have; the viewers
present rising or falling are present_join and present_leave, the sims'
join and leave a node's). A selection picks runs: `--src` (cache paths,
globs, default every one), `--cell`, `--arm` (globs) and `--seed`, and a
run's cell and arm are the same whatever else it selects. A suite farm
run's runs are named by the suite cell their matrix ran, <cell>@<run>,
which `--cell` matches whole or by the cell alone, and by the arm
suite.py names them by (arm_label): a `--sets` run's (its index has a
knob) by its set, the value its env gives the knob, else by its matrix's
arm, a `--refs` name. A farm run's cells hold every row it scored: also
those the farm reused from other runs or matrices, by its matrices'
indexes (target/sim-farm/<matrix>/index.jsonl), whose sources' tables
join the selection (pulled when the cache lacks them). A key's tables come
from one of its sources whatever cells it counts in (its copies are one
job's): the one holding the most of them, then the later flush (its
manifest's `at`), then a farm flush's later seq, by number
(source_ranks).

  sql      a query over the views (runs, scalars, seconds, viewer_seconds,
           reads, stalls, bins, viewers, events, parents, node_bins,
           forecast, the derived ones, and macros.sql's), their keys and
           joins below
  pooled   read lags pooled over the selected runs, by cell and arm: the
           quantiles of every read, beside the mean of the runs' own p99s;
           `--by` splits them by phase, by `--bin` of read time, or by
           viewer, each group's columns its own (its runs those with a read
           in it, their p99s of those reads)
  aligned  bands across runs around an event (`--nth` occurrence, the
           first by default): per `--bin` from `--before` to `--after`
           seconds, the median and p10/p90 over runs of `--field`: a read
           lag (lag_p50, lag_p99, lag_max, reads), a verdict column (healthy,
           lag_q, hit_share, delivery), a `seconds` column, or a `bins` one
           (summed over the bin). The events are the derived ones and the
           sims' own (`events`: faults, uplink steps, competing uploads,
           arrivals, joins, first reads or frames, churn leaves, seed steps
           and copies, departures and the losses and re-links after them;
           `--node` picks one node's), a kind the selection lacks refused
           with the kinds it has. `aligned depart` anchors a run on its
           first departure, which opens the departures' window
           (`SIM_DEPART`, `STRIMS_SIM_DEPART`), `--nth 0` on each one, and
           `--per node` each viewer that left on its own. With `--per node` each
           node's own series is aligned on its own `--nth` occurrence of
           a sim's per-node event (a viewer's join, first read or frame,
           churn leave), the bands over every run's nodes (n counts
           them); the per-node fields are a node's reads' lags, its
           `node_bins` columns (summed over the bin) and its
           `viewer_seconds` lag_ms (the bin's worst second) and frames
           (summed). The verdict, `seconds` and `bins` columns are
           run-wide, as are the derived events: `--per node` refuses
           them, as `--per run` refuses a field only a node has. The
           forecast log's fields (LOG_FIELDS: forecast_n,
           forecast_err_p50, forecast_err_p90, forecast_abs_err_p50,
           forecast_abs_err_p90) are a bin's samples, per run or per node
           (the forecasting node's), as `forecast` judges them against
           the truth. `--nth 0` aligns on every occurrence, a window
           each (n counts windows); `--pool` takes the field over every
           window's rows of a cell and arm together, a bin, rather than
           a band over windows; `--detail` (a glob) and `--since` /
           `--until` pick the events by their detail and time
  events   the run metrics pooled: collapses, onset after a step down and
           recovery after a step up (with how many runs they censored),
           excess lag, flaps
  osc      oscillation: spells and their dwell times, flaps, the lag's
           autocorrelation period and amplitude
  rows     the selected runs' rows (JSON lines), with their verdicts
  forecast the forecast logs pooled, by sim, cell, arm and the pace asked
           of the HAVEs: the error of each neighbour's announcement-lag
           estimate against the HAVE (sample - mean) and its horizon's
           coverage, and where the sim knows when an honest neighbour
           truly held the chunk as it arrived (`live`), the error against
           that
           (held - (seen + mean)), the HAVE's lateness over it (seen +
           sample - held: the HAVE's hold and its one-way delay), the
           share never held (censored), and the share the peer held before
           our channel with it opened (backlog: its first HAVEs announce
           them, so they don't count as forecasts of arrival).
           Only each HAVE's first sample of a stream counts, and not those
           that set the chunk's first sight (they are zero)

Every query takes `--json`: its rows, integer and decimal values (HUGEINT,
which sum() of integers gives, too) as numbers. The DuckDB CLI is pinned
(DUCKDB_VERSION, its sha256s) and fetched once into ~/.cache/strims/duckdb/
by `duckdb`, or by whatever needs it first.
"""

import argparse
import collections
import contextlib
import fcntl
import functools
import gzip
import hashlib
import importlib.util
import inspect
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# The views' keys and joins: `sql --help`, and the end of `--help`.
SQL_DOC = """\
Every view (macros.sql's too) has a run's key `k` and its source `src`, and
a key's tables come from one source, so any two views join on them:
`JOIN runs USING (k, src)` gives a row its run's cell, arm and seed, once a
cell (a key the farm reused counts in each of its cells). USING (k) joins
the same rows but keeps both `src` columns, to name as runs.src. Beside
(k, src) the views join on their own columns: reads and viewer_seconds to
viewers USING (k, src, viewer), as stalls does; events, parents, node_bins
and forecast to viewers USING (k, src, node) (`node` in both sims;
strims-sim's viewers also name it `host`); seconds to verdict_seconds USING (k, src, sec) (macros.sql's
`sec`). runs' frames and bytes are its row's: beside viewer_seconds' frames,
name them runs.frames and runs.bytes.

  simdb.py sql "SELECT r.cell, r.arm, w.up, quantile_cont(lag_ms, 0.99) AS p99
    FROM reads JOIN viewers w USING (k, src, viewer) JOIN runs r USING (k, src)
    GROUP BY ALL ORDER BY ALL"
"""
DUCKDB_VERSION = "v1.5.6"
DUCKDB_URL = f"https://github.com/duckdb/duckdb/releases/download/{DUCKDB_VERSION}/duckdb_cli-linux-amd64.gz"
DUCKDB_GZ_SHA256 = "cce2687a38681db644c248198b99ecd86170ff47e324c5f33e50c0042f142839"
DUCKDB_SHA256 = "61238cfbe9dfeaad4bcfcd8a48f6f7123dae2e9603aaadb1f77a4217aeb8980c"

# The sims' tables, and the store's own.
TABLES = ("seconds", "viewer_seconds", "reads", "stalls", "bins", "viewers", "events", "parents", "node_bins",
          "forecast")
# The bulky tables: the farm uploads them under sim-samples/, and a pull
# fetches them only when asked (`--reads`, or a query that reads them).
SAMPLE_TABLES = ("reads", "forecast")
# The most a query pulls of them by default (`--pull-mb`).
PULL_MB = 1000
RUN_COLS = {
    "k": "VARCHAR", "src": "VARCHAR", "sim": "VARCHAR", "test": "VARCHAR", "cell": "VARCHAR", "arm": "VARCHAR",
    "seed": "BIGINT", "env": "VARCHAR", "trace": "VARCHAR", "frames": "BIGINT", "bytes": "BIGINT",
    "wall_s": "DOUBLE", "rss_mb": "DOUBLE", "at": "VARCHAR", "bin_ms": "BIGINT", "wrote": "VARCHAR",
    "meta": "VARCHAR", "row_json": "VARCHAR",
}
SCALAR_COLS = {"k": "VARCHAR", "src": "VARCHAR", "name": "VARCHAR", "value": "DOUBLE"}
# series_meta.json's keys that aren't the series'.
META_ONLY = {"sim", "seconds", "viewers", "bin_ms"}
# Schema files' type names, as DuckDB's.
TYPES = {"int": "BIGINT", "integer": "BIGINT", "bigint": "BIGINT", "i64": "BIGINT", "u64": "BIGINT",
         "double": "DOUBLE", "float": "DOUBLE", "f64": "DOUBLE", "real": "DOUBLE",
         "varchar": "VARCHAR", "text": "VARCHAR", "string": "VARCHAR", "str": "VARCHAR",
         "bool": "BOOLEAN", "boolean": "BOOLEAN"}


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


@functools.cache
def main_checkout():
    """The main checkout, whose target/ holds the cache every worktree
    shares."""
    return Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).parent


def cache_root():
    """The cache's root: `SIM_DB_ROOT`, else the main checkout's
    target/sim-db."""
    if os.environ.get("SIM_DB_ROOT"):
        return Path(os.environ["SIM_DB_ROOT"])
    return main_checkout() / "target" / "sim-db"


_analyze = None


def analyze():
    """hack/capture/analyze.py: the verdicts, defined once."""
    global _analyze
    if _analyze is None:
        spec = importlib.util.spec_from_file_location("analyze", ROOT / "hack" / "capture" / "analyze.py")
        _analyze = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_analyze)
    return _analyze


# --- duckdb -------------------------------------------------------------------


def duckdb_bin(fetch=True):
    """The pinned DuckDB CLI, fetched and checked once; `SIM_DUCKDB` when
    set (a farm worker's, shipped as one of its run's binaries)."""
    if os.environ.get("SIM_DUCKDB"):
        return Path(os.environ["SIM_DUCKDB"])
    home = Path(os.environ.get("XDG_CACHE_HOME") or Path.home() / ".cache")
    path = home / "strims" / "duckdb" / DUCKDB_VERSION / "duckdb"
    if path.exists():
        return path
    if not fetch:
        sys.exit(f"no DuckDB at {path}: run hack/sim-db/simdb.py duckdb")
    path.parent.mkdir(parents=True, exist_ok=True)
    with urllib.request.urlopen(DUCKDB_URL, timeout=300) as r:
        gz = r.read()
    if hashlib.sha256(gz).hexdigest() != DUCKDB_GZ_SHA256:
        sys.exit(f"{DUCKDB_URL}: sha256 {hashlib.sha256(gz).hexdigest()}, not {DUCKDB_GZ_SHA256}")
    data = gzip.decompress(gz)
    if hashlib.sha256(data).hexdigest() != DUCKDB_SHA256:
        sys.exit(f"{DUCKDB_URL} unpacked: sha256 {hashlib.sha256(data).hexdigest()}, not {DUCKDB_SHA256}")
    tmp = path.with_suffix(f".{os.getpid()}")
    tmp.write_bytes(data)
    tmp.chmod(0o755)
    tmp.replace(path)
    return path


class DuckError(RuntimeError):
    pass


def duck(script, db=None, out="json", threads=None, memory=None):
    """Runs a SQL script in the DuckDB CLI, over `db` (a file) or in
    memory, and returns its last statement's rows (`out` "json"), its text
    (`out` "text": DuckDB's tables), or nothing (None).

    The CLI prints the values of some integer types (HUGEINT, which sum()
    of integers gives, UBIGINT, UHUGEINT) and of DECIMAL as JSON strings,
    to keep their precision. Rows come back with them as numbers: before
    a last statement that is a query (QUERY_WORDS) the script DESCRIBEs
    it, and its columns' types say which strings are numbers. A script
    the DESCRIBE fails runs as it is, its rows as the CLI prints them."""
    pre = []
    if threads:
        pre.append(f"SET threads={int(threads)};")
    if memory:
        pre.append(f"SET memory_limit='{memory}';")
    tmpdir = cache_root() / "tmp"
    if tmpdir.parent.exists():
        tmpdir.mkdir(exist_ok=True)
        pre.append(f"SET temp_directory={q(str(tmpdir))};")
    args = [str(duckdb_bin()), "-bail", "-batch", "-no-init"]
    if out == "json":
        args.append("-json")
    elif out == "text":
        args.append("-table")
    else:
        args.append("-noheader")
    args.append(str(db) if db else ":memory:")

    def run(s):
        return subprocess.run(args, input="\n".join(pre) + "\n" + s, capture_output=True, text=True)

    typed = described(script) if out == "json" else None
    p = run(typed or script)
    if p.returncode != 0 and typed:
        typed = None
        p = run(script)
    if p.returncode != 0:
        raise DuckError(p.stderr.strip() or p.stdout.strip()[-2000:])
    if out == "json":
        # Each statement with rows prints its own array (an empty one
        # without any): the last one's, after the DESCRIBE's.
        dec, at, arrays = json.JSONDecoder(object_pairs_hook=Pairs), 0, []
        text = p.stdout
        while True:
            while at < len(text) and text[at].isspace():
                at += 1
            if at >= len(text):
                break
            x, at = dec.raw_decode(text, at)
            arrays.append(x)
        if not arrays:
            return []
        if typed and len(arrays) >= 2:
            cols = [(c["column_name"], c["column_type"]) for c in plain(arrays[-2])]
            return [typed_row(r, cols) for r in arrays[-1]]
        return plain(arrays[-1])
    if out == "text":
        return p.stdout
    return None


# The statements duck() DESCRIBEs: queries, which a subquery can wrap.
QUERY_WORDS = {"SELECT", "WITH", "FROM", "VALUES", "DESCRIBE", "SUMMARIZE", "SHOW", "PIVOT", "UNPIVOT", "("}
# Integer types, some of which the CLI prints as JSON strings.
INT_TYPES = {"TINYINT", "SMALLINT", "INTEGER", "BIGINT", "HUGEINT", "UTINYINT", "USMALLINT", "UINTEGER",
             "UBIGINT", "UHUGEINT", "BIGNUM", "VARINT"}
SQL_SPECIAL = re.compile(r"""['";$]|--|/\*""")
DOLLAR_QUOTE = re.compile(r"\$[A-Za-z_]*\$")
FIRST_WORD = re.compile(r"(?:\s|--[^\n]*(?:\n|$)|/\*.*?\*/)*(\(|[A-Za-z]+)", re.S)


def statements(script):
    """The spans (start, end) of `script`'s statements, their semicolons
    left out, but those holding only whitespace and comments: quotes,
    quoted identifiers, dollar-quoted strings and comments read past."""
    spans, start, seen, i, n = [], 0, False, 0, len(script)
    while True:
        m = SQL_SPECIAL.search(script, i)
        j = m.start() if m else n
        seen |= j > i and not script[i:j].isspace()
        if not m:
            break
        tok = m.group()
        if tok in ("'", '"'):
            k = j + 1
            while 0 <= (k := script.find(tok, k)) and script.startswith(tok, k + 1):
                k += 2
            i, seen = (n if k < 0 else k + 1), True
        elif tok == "--":
            k = script.find("\n", j)
            i = n if k < 0 else k + 1
        elif tok == "/*":
            k = script.find("*/", j + 2)
            i = n if k < 0 else k + 2
        elif tok == "$":
            d = DOLLAR_QUOTE.match(script, j)
            k = script.find(d.group(), d.end()) if d else j
            i, seen = (n if k < 0 else k + (len(d.group()) if d else 1)), True
        else:
            if seen:
                spans.append((start, j))
            start, seen, i = j + 1, False, j + 1
    if seen:
        spans.append((start, n))
    return spans


def described(script):
    """`script` with its last statement, a query, DESCRIBEd before it
    runs; None when it isn't one."""
    spans = statements(script)
    if not spans:
        return None
    start, end = spans[-1]
    last = script[start:end]
    m = FIRST_WORD.match(last)
    if not m or m.group(1).upper() not in QUERY_WORDS:
        return None
    return f"{script[:start]}\nDESCRIBE SELECT * FROM (\n{last}\n);\n{script[start:]}"


class Pairs(list):
    """A JSON object's members in order, a name the CLI repeats (a join's
    result can have two columns of one name) kept twice."""


def plain(x):
    """`x` with its objects (Pairs) as dicts, as json.loads gives them."""
    if isinstance(x, Pairs):
        return {k: plain(v) for k, v in x}
    if isinstance(x, list):
        return [plain(v) for v in x]
    return x


def number(v, typ):
    """A value of a column of type `typ` the CLI printed as a string, as a
    number when it is one."""
    if not isinstance(v, str):
        return v
    try:
        if typ in INT_TYPES:
            return int(v)
        if typ.startswith("DECIMAL"):
            return float(v) if "." in v else int(v)
    except ValueError:
        pass
    return v


def typed_row(row, cols):
    """A row the CLI printed (Pairs) as a dict, its values numbers where
    its columns' types (`cols`, the DESCRIBE's (name, type) in order) are
    integer or decimal ones."""
    if len(row) == len(cols):
        return {k: plain(number(v, t)) for (k, v), (_, t) in zip(row, cols)}
    types = dict(cols)
    return {k: plain(number(v, types.get(k, ""))) for k, v in row}


def q(s):
    """A SQL string literal."""
    return "'" + str(s).replace("'", "''") + "'"


def ident(s):
    return '"' + str(s).replace('"', '""') + '"'


def struct(cols):
    """A read_csv/read_json `columns` argument."""
    return "{" + ", ".join(f"{q(n)}: {q(t)}" for n, t in cols.items()) + "}"


@contextlib.contextmanager
def locked(path, block=True):
    """An exclusive flock on `path` (made if missing); with `block` false,
    yields False rather than wait for another holder."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "a") as f:
        try:
            fcntl.flock(f, fcntl.LOCK_EX | (0 if block else fcntl.LOCK_NB))
        except BlockingIOError:
            yield False
            return
        try:
            yield True
        finally:
            fcntl.flock(f, fcntl.LOCK_UN)


# --- fold and export ----------------------------------------------------------


def schema(path):
    """A `<table>.schema.json`'s columns, {name: DuckDB type}, in order."""
    j = json.loads(path.read_text())
    cols = j.get("columns", j) if isinstance(j, dict) else j
    if isinstance(cols, dict):
        cols = list(cols.items())
    out = {}
    for c in cols:
        name, typ = (c["name"], c["type"]) if isinstance(c, dict) else c
        out[name] = TYPES.get(str(typ).lower(), str(typ).upper())
    return out


class FoldError(RuntimeError):
    pass


def scalars(row, prefix=""):
    """A row's numbers and bools, (name, value), lists element by element
    (`name[i]`)."""
    for name, v in row.items():
        key = f"{prefix}{name}"
        if isinstance(v, bool):
            yield key, float(v)
        elif isinstance(v, (int, float)):
            if isinstance(v, float) and not math.isfinite(v):
                continue
            yield key, float(v)
        elif isinstance(v, list) and name != "series":
            for i, x in enumerate(v):
                if isinstance(x, (int, float)) and not isinstance(x, bool):
                    yield f"{key}[{i}]", float(x)


def fold(db, jobs, src, threads=2, memory="1GB"):
    """Adds `jobs` to the store `db` in one transaction, under `src`. A job
    is a dict: `k` (its key), `dir` (its output dir), and what `runs`
    keeps of it (`sim`, `test`, `cell`, `arm`, `seed`, `env`, `row`,
    `wall_s`, `rss_mb`). A job whose dir holds no tables while its row has
    a series ran a binary that writes none: FoldError. Returns the tables
    each job wrote, by key. Two jobs of one key in the batch (a farm job and
    its verify copy, whose tables are the same) fold once, as a refold
    would replace the first."""
    jobs = list({j["k"]: j for j in jobs}.values())
    if not jobs:
        return {}
    work = Path(tempfile.mkdtemp(prefix=f"{Path(db).stem}.fold-", dir=Path(db).parent))
    try:
        cols = {}
        wrote = {}
        runs, scal = [], []
        for j in jobs:
            d = Path(j["dir"])
            have = [t for t in TABLES if (d / f"{t}.csv").exists() and (d / f"{t}.schema.json").exists()]
            row = dict(j.get("row") or {})
            if not have and row.get("series"):
                raise FoldError(f"{j['k']}: its binary wrote no tables (built before the sims wrote them?)")
            row.pop("series", None)
            meta = json.loads((d / "series_meta.json").read_text()) if (d / "series_meta.json").exists() else {}
            wrote[j["k"]] = have
            for t in have:
                for name, typ in schema(d / f"{t}.schema.json").items():
                    cols.setdefault(t, {}).setdefault(name, typ)
            num = lambda x: x if isinstance(x, (int, float)) and not isinstance(x, bool) else None
            runs.append({
                "k": j["k"], "src": src, "sim": j.get("sim"), "test": j.get("test"), "cell": j.get("cell"),
                "arm": j.get("arm"), "seed": j.get("seed"), "env": json.dumps(j.get("env") or {}, sort_keys=True),
                "trace": None if row.get("trace") is None else str(row.get("trace")),
                "frames": num(row.get("frames")), "bytes": num(row.get("bytes")),
                "wall_s": j.get("wall_s"), "rss_mb": j.get("rss_mb"),
                "at": j.get("at") or time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "bin_ms": meta.get("bin_ms"), "wrote": ",".join(have), "meta": json.dumps(meta),
                "row_json": json.dumps(row),
            })
            scal += [(j["k"], name, v) for name, v in scalars(row)]
        (work / "runs.jsonl").write_text("".join(json.dumps(r) + "\n" for r in runs))
        with open(work / "scalars.csv", "w") as f:
            f.write("k,src,name,value\n")
            for k, name, v in scal:
                f.write(f"{csv_text(k)},{csv_text(src)},{csv_text(name)},{v!r}\n")
        keys = "(" + ", ".join(q(j["k"]) for j in jobs) + ")"
        sql = ["BEGIN;"]
        for t, c in [("runs", RUN_COLS), ("scalars", SCALAR_COLS)] + [
                (t, {"k": "VARCHAR", "src": "VARCHAR", **cols[t]}) for t in TABLES if t in cols]:
            first = ", ".join(f"{ident(n)} {typ}" for n, typ in c.items())
            sql.append(f"CREATE TABLE IF NOT EXISTS {t} ({first});")
            sql += [f"ALTER TABLE {t} ADD COLUMN IF NOT EXISTS {ident(n)} {typ};" for n, typ in c.items()]
            sql.append(f"DELETE FROM {t} WHERE k IN {keys};")
        sql.append(f"INSERT INTO runs BY NAME SELECT * FROM read_json({q(work / 'runs.jsonl')}, "
                   f"format='newline_delimited', columns={struct(RUN_COLS)});")
        sql.append(f"INSERT INTO scalars BY NAME SELECT * FROM read_csv({q(work / 'scalars.csv')}, header=true, "
                   f"auto_detect=false, columns={struct(SCALAR_COLS)});")
        for j in jobs:
            d = Path(j["dir"])
            for t in wrote[j["k"]]:
                c = schema(d / f"{t}.schema.json")
                sql.append(f"INSERT INTO {t} BY NAME SELECT {q(j['k'])} AS k, {q(src)} AS src, * FROM "
                           f"read_csv({q(d / f'{t}.csv')}, header=true, auto_detect=false, columns={struct(c)});")
        sql.append("COMMIT;")
        duck("\n".join(sql), db, out=None, threads=threads, memory=memory)
        return wrote
    finally:
        shutil.rmtree(work, ignore_errors=True)


def csv_text(s):
    s = str(s)
    return '"' + s.replace('"', '""') + '"' if any(c in s for c in ',"\n\r') else s


def sha256(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def tables_in(db):
    rows = duck("SELECT table_name AS t FROM information_schema.tables WHERE table_schema = 'main' ORDER BY 1;",
                db)
    return [r["t"] for r in rows]


def export(db, dest, threads=None):
    """Writes the store `db` to `dest` (made whole: written beside it, then
    renamed), a zstd Parquet file a table and `manifest.json`; returns the
    manifest, or None for an empty store."""
    dest = Path(dest)
    have = tables_in(db)
    if "runs" not in have:
        return None
    counts = duck(" UNION ALL ".join(f"SELECT {q(t)} AS t, count(*) AS n FROM {t}" for t in have) + ";", db)
    counts = {r["t"]: r["n"] for r in counts}
    if not counts.get("runs"):
        return None
    tmp = dest.with_name(dest.name + f".tmp{os.getpid()}")
    shutil.rmtree(tmp, ignore_errors=True)
    tmp.mkdir(parents=True)
    sql = [f"COPY {t} TO {q(tmp / f'{t}.parquet')} (FORMAT parquet, COMPRESSION zstd);" for t in have if counts[t]]
    duck("\n".join(sql), db, out=None, threads=threads)
    keys = [r["k"] for r in duck("SELECT k FROM runs ORDER BY k;", db)]
    manifest = {
        "keys": keys, "at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "tables": {t: {"rows": counts[t], "bytes": (tmp / f"{t}.parquet").stat().st_size,
                       "sha256": sha256(tmp / f"{t}.parquet")}
                   for t in have if counts[t]},
    }
    (tmp / "manifest.json").write_text(json.dumps(manifest, indent=1) + "\n")
    if dest.exists():
        shutil.rmtree(dest)
    dest.parent.mkdir(parents=True, exist_ok=True)
    tmp.rename(dest)
    return manifest


def recover(store_dir, dest_of):
    """Exports and removes the stores in `store_dir` that no process holds
    (a crashed invocation's: `<inv>.duckdb` beside its `<inv>.lock`), each
    to `dest_of(inv)`; returns the invocations exported."""
    done = []
    for db in sorted(Path(store_dir).glob("*.duckdb")):
        inv = db.stem
        with locked(db.with_suffix(".lock"), block=False) as mine:
            if not mine:
                continue
            try:
                if export(db, dest_of(inv)):
                    done.append(inv)
            except DuckError as e:
                print(f"store {db}: {e}", file=sys.stderr)
            for p in (db, db.with_suffix(".duckdb.wal")):
                p.unlink(missing_ok=True)
            for p in db.parent.glob(f"{inv}.fold-*"):
                shutil.rmtree(p, ignore_errors=True)
        db.with_suffix(".lock").unlink(missing_ok=True)
    return done


# --- rows ---------------------------------------------------------------------


def files(srcs, table):
    """`table`'s Parquet files in the cache's sources `srcs`."""
    root = cache_root() / "cache"
    return [str(root / s / f"{table}.parquet") for s in srcs if (root / s / f"{table}.parquet").exists()]


def plist(paths):
    return "[" + ", ".join(q(p) for p in paths) + "]"


def series_of(meta, seconds, viewer_seconds, stays=None):
    """A run's series as its row's `series` has it, from its tables: the
    meta's fields, each `seconds` column (a list by second), and per viewer
    `lag` and `frames`; with `stays` (the viewers table's rows: viewer,
    arrive_ms, depart_ms), per viewer `arrive_ms` and `depart_ms`, the
    stays analyze.presence reads (None for a departure that didn't come)."""
    if not seconds:
        return None
    # A row's series leaves `hostile` out without any.
    s = {k: v for k, v in meta.items() if k not in META_ONLY and not (k == "hostile" and not v)}
    seconds = sorted(seconds, key=lambda r: r["sec"])
    for c in seconds[0]:
        if c in ("k", "src", "sec"):
            continue
        v = [r.get(c) for r in seconds]
        if any(x is not None for x in v):
            s[c] = v
    by = sorted(viewer_seconds, key=lambda r: r["viewer"])
    # strims-sim writes no viewer_seconds row for a second its viewer
    # wasn't there in: the series' -1 frames (and lag) there.
    n = len(seconds)
    def by_sec(r, f):
        out = [-1] * n
        for k, x in zip(r.get("sec") or range(len(r[f])), r[f]):
            if 0 <= k < n:
                out[k] = x
        return out
    s["lag"] = [by_sec(r, "lag") for r in by]
    s["frames"] = [by_sec(r, "frames") for r in by]
    if stays:
        at = {r["viewer"]: r for r in stays}
        if all(r["viewer"] in at for r in by):
            s["arrive_ms"] = [at[r["viewer"]]["arrive_ms"] for r in by]
            s["depart_ms"] = [at[r["viewer"]]["depart_ms"] for r in by]
    return s


def wanted(pairs):
    """A VALUES list of (src, k) for a join."""
    return "(VALUES " + ", ".join(f"({q(s)}, {q(k)})" for s, k in pairs) + ") w(src, k)"


def load_series(pairs):
    """{(src, k): (meta, series)} for `pairs` of (src, k), from the cache."""
    pairs = sorted(set(pairs))
    srcs = sorted({s for s, _ in pairs})
    runs = files(srcs, "runs")
    if not pairs or not runs:
        return {}
    want = wanted(pairs)
    meta = {(r["src"], r["k"]): json.loads(r["meta"] or "{}") for r in duck(
        f"SELECT src, k, meta FROM read_parquet({plist(runs)}, union_by_name=true) SEMI JOIN {want} USING (src, k);")}
    secs = collections.defaultdict(list)
    if files(srcs, "seconds"):
        for r in duck(f"SELECT * FROM read_parquet({plist(files(srcs, 'seconds'))}, union_by_name=true) "
                      f"SEMI JOIN {want} USING (src, k);"):
            secs[(r["src"], r["k"])].append(r)
    vs = collections.defaultdict(list)
    if files(srcs, "viewer_seconds"):
        for r in duck(f"SELECT src, k, viewer, list(sec ORDER BY sec) AS sec, list(lag_ms ORDER BY sec) AS lag, "
                      f"list(frames ORDER BY sec) AS frames "
                      f"FROM read_parquet({plist(files(srcs, 'viewer_seconds'))}, union_by_name=true) "
                      f"SEMI JOIN {want} USING (src, k) GROUP BY ALL;"):
            vs[(r["src"], r["k"])].append(r)
    # The viewers' stays, where the tables have them (both sims' since
    # their arrivals and departures).
    stays = collections.defaultdict(list)
    viewers = files(srcs, "viewers")
    if viewers:
        table = f"read_parquet({plist(viewers)}, union_by_name=true)"
        cols = {r["column_name"] for r in duck(f"DESCRIBE SELECT * FROM {table};")}
        if {"arrive_ms", "depart_ms"} <= cols:
            for r in duck(f"SELECT src, k, viewer, arrive_ms, depart_ms FROM {table} "
                          f"SEMI JOIN {want} USING (src, k);"):
                stays[(r["src"], r["k"])].append(r)
    return {p: (m, series_of(m, secs.get(p, []), vs.get(p, []), stays.get(p))) for p, m in meta.items()}


def judge(s):
    """A series' verdict and run metrics (analyze.py), or (None, None)."""
    if not s:
        return None, None
    a = analyze()
    try:
        v = a.verdict(s)
    except Exception:
        return None, None
    return v, a.run_metrics(s, v)


def rows(pairs, series=False):
    """The rows of `pairs` of (src, k) in the cache, {(src, k): row}: each
    the sim's row (less its series) with `_verdict` and `_metrics`
    (analyze.py) from its tables' series, `_wall_s` and `_rss_mb`, and
    with `series` the series as `_series`."""
    pairs = sorted(set(pairs))
    runs = files(sorted({s for s, _ in pairs}), "runs")
    if not pairs or not runs:
        return {}
    got = load_series(pairs)
    out = {}
    for r in duck(f"SELECT src, k, row_json, wall_s, rss_mb FROM read_parquet({plist(runs)}, union_by_name=true) "
                  f"SEMI JOIN {wanted(pairs)} USING (src, k);"):
        row = json.loads(r["row_json"])
        row["_wall_s"], row["_rss_mb"] = r["wall_s"], r["rss_mb"]
        _, s = got.get((r["src"], r["k"]), ({}, None))
        row["_verdict"], row["_metrics"] = judge(s)
        if series:
            row["_series"] = s
        out[(r["src"], r["k"])] = row
    return out


def have_src(src):
    """Whether the cache holds source `src`'s export."""
    return (cache_root() / "cache" / src / "runs.parquet").exists()


# --- derived tables -----------------------------------------------------------

DERIVED = {
    "verdict_seconds": {"k": "VARCHAR", "src": "VARCHAR", "sec": "BIGINT", "phase": "VARCHAR", "healthy": "BOOLEAN",
                        "trees_bad": "BOOLEAN", "deliv_bad": "BOOLEAN", "lag_high": "BOOLEAN", "lag_rise": "BOOLEAN",
                        "hit_share": "DOUBLE", "delivery": "DOUBLE", "lag_q": "DOUBLE"},
    "episodes": {"k": "VARCHAR", "src": "VARCHAR", "kind": "VARCHAR", "start": "BIGINT", "stop": "BIGINT",
                 "len": "BIGINT"},
    "derived_events": {"k": "VARCHAR", "src": "VARCHAR", "sec": "DOUBLE", "kind": "VARCHAR", "value": "DOUBLE"},
}
# derived_events' kinds (analyze.events'), none of them the sims'.
DERIVED_KINDS = ("start", "seed_down", "seed_up", "present_join", "present_leave", "onset", "recovered", "collapse")
# A run's verdict, then analyze.run_metrics' columns. startup_s is the
# seconds a collapse must last past (analyze.startup).
VERDICT_COLS = {"k": "VARCHAR", "src": "VARCHAR", "onset": "BIGINT", "trees": "BIGINT", "delivery": "BIGINT",
                "latency": "BIGINT", "recovered": "BIGINT", "absorbing": "BOOLEAN", "feasible": "VARCHAR",
                "unhealthy": "DOUBLE", "lag_p50": "DOUBLE", "lag_max": "DOUBLE", "collapsed": "BOOLEAN",
                "startup_s": "BIGINT"}


@functools.cache
def derived_version():
    """A hash of everything that defines the derived tables: analyze.py's
    VERDICT_VERSION, their columns and the code that writes them
    (DERIVE_CODE), so a derived table made under another is stale."""
    parts = [analyze().VERDICT_VERSION, repr([DERIVED, DERIVED_KINDS, VERDICT_COLS])]
    parts += [inspect.getsource(f) for f in DERIVE_CODE]
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()[:12]


def derived_dir(table):
    """Derived `table`'s files: a source's each, under derived_version."""
    return cache_root() / "derived" / table / f"v={derived_version()}"


def derive(srcs):
    """Makes the derived tables of every source in `srcs` that lacks them
    under the current derived_version."""
    a = analyze()
    for src in srcs:
        name = src.replace("/", "~") + ".parquet"
        if all((derived_dir(t) / name).exists() for t in [*DERIVED, "verdicts"]):
            continue
        with locked(cache_root() / "derive.lock"):
            if all((derived_dir(t) / name).exists() for t in [*DERIVED, "verdicts"]):
                continue
            keys = [r["k"] for r in duck(f"SELECT k FROM read_parquet({plist(files([src], 'runs'))});")]
            got = load_series([(src, k) for k in keys])
            out = {t: [] for t in DERIVED}
            verdicts = []
            metric_names = None
            for (_, k), (meta, s) in got.items():
                v, m = judge(s)
                if v is None:
                    continue
                if metric_names is None:
                    metric_names = list(m)
                ph = a.phases(s, v)
                for sec in range(len(v["healthy"])):
                    out["verdict_seconds"].append([k, src, sec, ph[sec], v["healthy"][sec], v["trees_bad"][sec],
                                                   v["deliv_bad"][sec], v["lag_high"][sec], v["lag_rise"][sec],
                                                   v["hit_share"][sec], v["delivery_share"][sec], v["lag_q"][sec]])
                out["episodes"] += [[k, src, kind, x, y, y - x] for kind, x, y in a.episodes(v)]
                out["derived_events"] += [[k, src, t, kind, value] for t, kind, value in a.events(s, v)]
                verdicts.append([k, src, v["onset"], v["trees"], v["delivery"], v["latency"], v["recovered"],
                                 v["absorbing"], str(v["feasible"]), v["unhealthy"], v["lag_p50"], v["lag_max"],
                                 a.collapsed(v), a.run_startup(v)] + [m[n] for n in metric_names])
            cols = dict(VERDICT_COLS)
            for n in metric_names or []:
                cols[n] = "BOOLEAN" if n.endswith("_censored") else "DOUBLE"
            write_table(derived_dir("verdicts") / name, cols, verdicts)
            for t, c in DERIVED.items():
                write_table(derived_dir(t) / name, c, out[t])


def write_table(path, cols, data):
    """Writes rows `data` (lists in `cols`' order) to Parquet at `path`,
    its columns `cols` alone: the CSV it's read from sits in the derived
    dir's `v=` directory, which DuckDB would take for a hive partition and
    add as a column."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", suffix=".csv", dir=path.parent, delete=False) as f:
        f.write(",".join(cols) + "\n")
        for r in data:
            f.write(",".join("" if x is None else csv_text(str(x).lower() if isinstance(x, bool) else x)
                             for x in r) + "\n")
        tmp = Path(f.name)
    part = path.with_suffix(f".tmp{os.getpid()}")
    try:
        duck(f"COPY (SELECT * FROM read_csv({q(tmp)}, header=true, auto_detect=false, columns={struct(cols)}, "
             f"nullstr='', hive_partitioning=false)) TO {q(part)} (FORMAT parquet, COMPRESSION zstd);", out=None)
        part.replace(path)
    finally:
        tmp.unlink(missing_ok=True)
        part.unlink(missing_ok=True)


# The code that writes the derived tables, whose source derived_version
# hashes.
DERIVE_CODE = (derive, write_table)


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


def sources(patterns):
    """The cache's sources (dirs holding runs.parquet) matching `patterns`."""
    root = cache_root() / "cache"
    out = set()
    for p in patterns or ["**"]:
        for m in root.glob(p):
            if (m / "runs.parquet").exists():
                out.add(str(m.relative_to(root)))
            for r in m.glob("**/runs.parquet"):
                out.add(str(r.parent.relative_to(root)))
    return sorted(out)


def suite_index(run):
    """Suite farm run `run`'s index: the main checkout's
    target/sim-suite/farm-runs/<run>.json, or while suite.py keeps it apart
    (written by `farm --no-run`, the run started by hand) pending/<run>.json
    there; {} if neither."""
    d = main_checkout() / "target" / "sim-suite" / "farm-runs"
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
    farm = main_checkout() / "target" / "sim-farm"
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
    held = [r["src"] for r in duck(f"SELECT DISTINCT src FROM {sql};")]
    return (sql if held else None), held


def source_ranks(srcs):
    """{src: rank} over the cache's sources `srcs`, the higher the rank the
    sooner the views take a key's tables from it: the source holding the
    most of the sims' tables (a farm run's reads come with `pull --reads`),
    then the later flush (its manifest's `at`), then a farm flush's later
    seq (farm/<run>/<instance>/<seq>, by number), then the name."""
    root = cache_root() / "cache"

    def key(s):
        last = s.rsplit("/", 1)[-1]
        seq = int(last) if s.startswith("farm/") and last.isdigit() else -1
        return sum((root / s / f"{t}.parquet").exists() for t in TABLES), manifest_of(s).get("at") or "", seq, s

    return {s: n for n, s in enumerate(sorted(srcs, key=key))}


def views(a, derived=True, fetch=True):
    """The SQL that makes the views over selection `a`'s sources, and the
    tables it has. A farm source's runs name the suite cells and arms its
    matrices ran (suite_cells), and a farm run's cells hold the rows it
    reused (reused: their sources' tables join the views, pulled into the
    cache when it lacks them, unless not `fetch`); `matrix` is the matrix
    (or local cell) a run counts for."""
    srcs = sources(a.src)
    if not srcs:
        sys.exit("no runs in the cache match --src")
    stored = f"read_parquet({plist(files(srcs, 'runs'))}, union_by_name=true)"
    moved, held = None, []
    if any(s.startswith("farm/") for s in srcs):
        moved, held = reused(srcs, [r["cell"] for r in duck(
            f"SELECT DISTINCT cell FROM {stored} WHERE src LIKE 'farm/%';")])
    lack = [s for s in held if not have_src(s)]
    if lack and not fetch:
        print(f"the cache lacks {len(lack)} flushes whose rows the selection's runs reused: left out",
              file=sys.stderr)
    elif lack:
        print(f"pulling the farm runs of {len(lack)} flushes whose rows the selection's runs reused",
              file=sys.stderr)
        gone = pull_sources(lack)
        if gone:
            print(f"{len(gone)} of them are gone from S3 (a week's): their rows are left out", file=sys.stderr)
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
        cells = suite_cells([r["matrix"] for r in duck(f"SELECT DISTINCT matrix FROM {read_runs};")])
    if cells:
        pairs = ", ".join(f"({q(n)}, {q(c)}, {q(k) if k else 'NULL'})" for n, (c, k) in cells.items())
        # The arm as arm_label names it.
        read_runs = (
            f"(SELECT r.* REPLACE (coalesce(m.cell, r.cell) AS cell, CASE WHEN m.knob IS NULL THEN r.arm "
            f"ELSE coalesce(json_extract_string(r.env, '$.' || m.knob), r.arm) END AS arm) "
            f"FROM {read_runs} r LEFT JOIN (VALUES {pairs}) m(matrix, cell, knob) ON r.matrix = m.matrix)"
        )
    where = []
    if a.cell:
        where.append("(" + " OR ".join(f"cell GLOB {q(c)} OR split_part(cell, '@', 1) GLOB {q(c)}"
                                       for c in a.cell) + ")")
    if a.arm:
        where.append("(" + " OR ".join(f"arm GLOB {q(x)}" for x in a.arm) + ")")
    if a.seed:
        where.append(f"seed IN ({', '.join(str(int(s)) for s in a.seed)})")
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
                got = {r["column_name"] for r in duck(f"DESCRIBE SELECT * FROM read_parquet({plist(f)}, "
                                                      "union_by_name=true);")}
                if "host" in got:
                    cols = "x.* REPLACE (coalesce(x.node, x.host) AS node)" if "node" in got else "x.*, x.host AS node"
            sql.append(f"CREATE VIEW {t} AS SELECT {cols} FROM read_parquet({plist(f)}, union_by_name=true) x "
                       "SEMI JOIN runs USING (k, src);")
            have.add(t)
    if derived:
        for t in [*DERIVED, "verdicts"]:
            if f := derived_files(t, srcs):
                sql.append(f"CREATE VIEW {t} AS SELECT x.* FROM {f} SEMI JOIN runs USING (k, src);")
                have.add(t)
    sql += macros(have)
    return "\n".join(sql), have


def macros(have):
    """macros.sql's statements whose `-- needs:` tables the views have."""
    out, needs, cur = [], set(), []
    for line in (Path(__file__).parent / "macros.sql").read_text().splitlines():
        if line.startswith("-- needs:"):
            needs = set(line.split(":", 1)[1].split())
        cur.append(line)
        if line.rstrip().endswith(";"):
            if needs <= have:
                out.append("\n".join(cur))
            cur, needs = [], set()
    return out


def need(have, *tables):
    missing = [t for t in tables if t not in have]
    if missing:
        sys.exit(f"the selection has no {', '.join(missing)}"
                 + (" (nor does S3: run with the reads)" if "reads" in missing else "")
                 + (" (nor does S3: run with SIM_FORECAST=1 / STRIMS_SIM_FORECAST=1)" if "forecast" in missing else ""))


def with_samples(a, v, have, tables, derived=True):
    """The views of selection `a` (`v`, `have`) once the cache holds what S3
    has of its jobs' sample `tables` (pull_samples, at most `--pull-mb`)."""
    if pull_samples(v, tables, limit_mb=getattr(a, "pull_mb", PULL_MB)):
        return views(a, derived)
    return v, have


def emit(a, rows_, text):
    if a.json:
        print(json.dumps(rows_))
    else:
        print(text())


def short(arm):
    """An arm label for a table's header."""
    return arm if arm is None or len(arm) <= 24 else arm[:10] + "..." + arm[-11:]


# --- queries ------------------------------------------------------------------


def time_filter(a, col="t_ms"):
    w = []
    if a.since is not None:
        w.append(f"{col} >= {int(a.since * 1000)}")
    if a.until is not None:
        w.append(f"{col} < {int(a.until * 1000)}")
    return w


def cmd_sql(a):
    v, have = views(a)
    if getattr(a, "reads", False):
        v, _ = with_samples(a, v, have, SAMPLE_TABLES)
    out = duck(v + "\n" + a.query + ("" if a.query.rstrip().endswith(";") else ";"),
               out="json" if a.json else "text")
    print(json.dumps(out) if a.json else out, end="" if not a.json else "\n")


def fmt(x, nd=0):
    if x is None:
        return "-"
    if isinstance(x, float):
        return f"{x:.{nd}f}"
    return str(x)


def cmd_pooled(a):
    v, have = with_samples(a, *views(a), ("reads",))
    need(have, "reads")
    group = {"none": "NULL", "bin": f"(t_ms // {a.bin}) * {a.bin}", "viewer": "viewer",
             "phase": "coalesce(vs.phase, 'tail')"}[a.by]
    join = ("LEFT JOIN verdict_seconds vs ON vs.k = x.k AND vs.src = x.src AND vs.sec = x.t_ms // 1000"
            if a.by == "phase" else "")
    where = time_filter(a, "x.t_ms")
    # Every column is the group's: its runs are those with a read in it,
    # and each run's p99 is of its reads there.
    same = " AND ".join(f"pool.{c} IS NOT DISTINCT FROM runp.{c}" for c in ("cell", "arm", "g"))
    sql = f"""
WITH x AS (SELECT r.cell, r.arm, {group} AS g, x.* FROM reads x JOIN runs r USING (k, src) {join}
           {"WHERE " + " AND ".join(where) if where else ""}),
per AS (SELECT cell, arm, g, k, src, quantile_cont(lag_ms, 0.99) AS p99 FROM x GROUP BY ALL),
runp AS (SELECT cell, arm, g, avg(p99) AS mean_run_p99, count(*) AS runs FROM per GROUP BY ALL),
pool AS (SELECT cell, arm, g, count(*) AS reads, quantile_cont(lag_ms, [0.5, 0.9, 0.99]) AS q, max(lag_ms) AS lag_max
         FROM x GROUP BY ALL)
SELECT pool.cell, pool.arm, pool.g, runs, reads, q[1] AS lag_p50, q[2] AS lag_p90, q[3] AS lag_p99, lag_max,
       mean_run_p99
FROM pool JOIN runp ON {same} ORDER BY pool.cell, pool.arm, pool.g;"""
    out = duck(v + sql)

    def text():
        lines = [f"{'cell':22} {'arm':24} {a.by if a.by != 'none' else '':>10} {'runs':>5} {'reads':>9} "
                 f"{'p50':>7} {'p90':>7} {'p99':>7} {'max':>7}  mean of runs' p99"]
        for r in out:
            lines.append(f"{r['cell'] or '-':22} {short(r['arm']) or '-':24} {fmt(r['g']):>10} {r['runs']:5d} "
                         f"{r['reads']:9d} {fmt(r['lag_p50']):>7} {fmt(r['lag_p90']):>7} {fmt(r['lag_p99']):>7} "
                         f"{fmt(r['lag_max']):>7}  {fmt(r['mean_run_p99'])}")
        return "\n".join(lines)

    emit(a, out, text)


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
# forecast's node is a viewer), as cmd_forecast judges them against the
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


def field_source(a, v, have):
    """Where `--field` comes from: (series, a table or a query; time column
    in ms; aggregate). Per node (`--per node`) each of the series' rows has
    its `node`: a viewer's reads and viewer_seconds by `viewers`."""
    f, per_node = a.field, a.per == "node"

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
        return {r["column_name"] for r in duck(v + f"\nDESCRIBE {t};")} - ID_COLS if t in have else set()

    def check_bin(t):
        unit = 1000 if t in ("seconds", "viewer_seconds") else (
            duck(v + "\nSELECT max(bin_ms) AS b FROM runs;")[0]["b"] or 1)
        if a.bin % unit:
            sys.exit(f"{f} comes in {unit} ms bins: --bin must be a multiple of {unit} "
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
            sys.exit(f"{f} is run-wide ({wide} column): --per node aligns a node's own fields, its reads' "
                     f"({', '.join(READ_FIELDS)}), viewer_seconds' ({', '.join(VIEWER_SECOND_FIELDS)}) and "
                     "node_bins' columns"
                     + ("" if "node_bins" in have else
                        " (the selection has no node_bins: SIM_BIN_NODES=1 / STRIMS_SIM_BIN_NODES=1 write them)"))
    else:
        if f in VERDICT_FIELDS:
            if a.bin % 1000:
                sys.exit(f"{f} is per second: --bin must be a multiple of 1000")
            return "verdict_seconds", "sec * 1000", VERDICT_FIELDS[f]
        for t, tcol in (("bins", "t_ms"), ("seconds", "sec * 1000")):
            if f in columns(t):
                check_bin(t)
                return t, tcol, f"sum({ident(f)})::DOUBLE"
        if nodes:
            sys.exit(f"{f} is a node's (a {nodes[0]} column): align it --per node")
    sys.exit(f"no field {f!r}: a read lag ({', '.join(READ_FIELDS)}), "
             + ("a viewer_seconds column or a node_bins one" if per_node else
                f"a verdict column ({', '.join(VERDICT_FIELDS)}), or a seconds or bins column"))


def cmd_aligned(a):
    v, have = views(a)
    if a.field in READ_FIELDS:
        v, have = with_samples(a, v, have, ("reads",))
    elif a.field in LOG_FIELDS:
        v, have = with_samples(a, v, have, (LOG_FIELDS[a.field][0],))
    need(have, "derived_events")
    per_node = a.per == "node"
    if per_node:
        need(have, "events")
    series, tcol, agg = field_source(a, v, have)
    lo, hi = int(-a.before * 1000), int(a.after * 1000)
    node = "" if a.node is None else f" AND node = {int(a.node)}"
    keys = "k, src, node" if per_node else "k, src"
    kind = f"kind = {q(a.event)}"
    sims = (f"(SELECT count(*) FROM events WHERE {kind}{node}) AS sims, "
            f"(SELECT count(*) FROM events WHERE {kind}{node} AND node >= 0) AS nodes"
            if "events" in have else "0 AS sims, 0 AS nodes")
    # A derived event has no node.
    derived = f"(SELECT count(*) FROM derived_events WHERE {kind}{'' if a.node is None else ' AND false'})"
    got = duck(v + f"\nSELECT {derived} AS derived, {sims}, "
                   "(SELECT list(DISTINCT kind ORDER BY kind) FROM ev) AS kinds;")[0]
    if got["derived"] and got["sims"]:
        sys.exit(f"{a.event} is both a derived event and a sim's: analyze.events must name its own apart "
                 "(DERIVED_KINDS)")
    if not got["derived"] and not got["sims"]:
        sys.exit(f"the selection has no {a.event} event" + (f" of node {a.node}" if a.node is not None else "")
                 + f"; its kinds: {', '.join(got['kinds'] or [])}")
    if per_node:
        if not got["nodes"]:
            sys.exit(f"{a.event} is run-wide (a derived event, or a sim's with no node): align it --per run")
        node += " AND node >= 0"
    if a.detail is not None:
        node += f" AND detail GLOB {q(a.detail)}"
    node += "".join(f" AND {w}" for w in time_filter(a))
    # The nth occurrence, or every one (`--nth 0`), each its own window.
    pick = "" if int(a.nth) == 0 else f" WHERE nth = {int(a.nth)}"
    sql = f"""
WITH e AS (SELECT {keys}, t_ms AS e_ms, nth AS eid FROM
              (SELECT *, row_number() OVER (PARTITION BY {keys} ORDER BY t_ms) AS nth
               FROM ev WHERE kind = {q(a.event)}{node}){pick}),
x AS (SELECT x.*, e.eid, floor(({tcol} - e.e_ms) / {a.bin}) * {a.bin} AS rel
      FROM {series} x JOIN e USING ({keys})
      WHERE {tcol} - e.e_ms >= {lo} AND {tcol} - e.e_ms < {hi}),"""
    if a.pool:
        # Every window's rows of a cell and arm in one pool a bin.
        sql += f"""
pool AS (SELECT r.cell, r.arm, x.rel, count(*) AS n, {agg} AS v FROM x JOIN runs r USING (k, src) GROUP BY ALL)
SELECT cell, arm, rel::BIGINT AS rel_ms, n, v AS p10, v AS p50, v AS p90 FROM pool WHERE v IS NOT NULL
ORDER BY cell, rel_ms, arm;"""
        out = duck(v + sql)
    else:
        sql += f"""
per AS (SELECT * FROM (SELECT {keys}, eid, rel, {agg} AS v FROM x GROUP BY ALL) WHERE v IS NOT NULL)
SELECT r.cell, r.arm, rel::BIGINT AS rel_ms, count(*) AS n, quantile_cont(v, [0.1, 0.5, 0.9]) AS q
FROM per JOIN runs r USING (k, src) GROUP BY ALL ORDER BY r.cell, rel_ms, r.arm;"""
        out = duck(v + sql)
        for r in out:
            r["p10"], r["p50"], r["p90"] = r.pop("q")
    if per_node and not out:
        print(f"no node with a {a.event} event has a {a.field} series around it", file=sys.stderr)

    def text():
        arms = sorted({r["arm"] for r in out})
        lines = []
        nth = "every one" if int(a.nth) == 0 else f"#{a.nth}"
        around = f"each node's own {a.event} ({nth})" if per_node else f"{a.event} ({nth})"
        over = "nodes of runs" if per_node else "runs"
        if int(a.nth) == 0:
            over = f"windows ({over}' every event)"
        if a.pool:
            over = "the windows' pooled rows (the pool's value thrice)"
        for cell in sorted({r["cell"] for r in out}):
            lines.append(f"\n{cell}: {a.field} around {around}, median [p10, p90] over {over} (n)")
            lines.append(f"{'rel_s':>8} " + " ".join(f"{short(x) or '-':>34}" for x in arms))
            by = {(r["rel_ms"], r["arm"]): r for r in out if r["cell"] == cell}
            for rel in sorted({k for k, _ in by}):
                cells = []
                for x in arms:
                    r = by.get((rel, x))
                    cells.append(f"{'-':>34}" if r is None else
                                 f"{fmt(r['p50'], 2):>10} [{fmt(r['p10'], 2)}, {fmt(r['p90'], 2)}] ({r['n']})".rjust(34))
                lines.append(f"{rel / 1000:8.2f} " + " ".join(cells))
        return "\n".join(lines)

    emit(a, out, text)


def cmd_events(a):
    v, have = views(a)
    need(have, "verdicts")
    sql = """
SELECT r.cell, r.arm, count(*) AS runs, sum(collapsed::INT)::BIGINT AS collapsed, sum(collapses)::BIGINT AS collapses,
       median(onset_after_drop_s) AS onset_after_drop_s, sum(onset_after_drop_censored::INT)::BIGINT AS onset_censored,
       count(onset_after_drop_s) AS drops,
       median(recover_after_rise_s) AS recover_after_rise_s, sum(recover_after_rise_censored::INT)::BIGINT AS recover_censored,
       count(recover_after_rise_s) AS rises,
       median(excess_lag_area_ms_s) AS excess_area_p50, quantile_cont(excess_lag_area_ms_s, 0.9) AS excess_area_p90,
       median(excess_lag_peak_ms) AS excess_peak_p50, median(flaps_per_min) AS flaps_per_min
FROM verdicts JOIN runs r USING (k, src) GROUP BY ALL ORDER BY ALL;"""
    out = duck(v + sql)

    def text():
        lines = [f"{'cell':22} {'arm':24} {'runs':>4} {'coll':>4} {'onset_s (cens/n)':>17} {'recover_s (cens/n)':>19} "
                 f"{'excess p50 p90':>15} {'peak':>6} {'flaps':>6}"]
        for r in out:
            lines.append(
                f"{r['cell'] or '-':22} {short(r['arm']) or '-':24} {r['runs']:4d} {r['collapsed'] or 0:4d} "
                f"{fmt(r['onset_after_drop_s'], 1):>7} ({r['onset_censored'] or 0}/{r['drops']})".ljust(73)
                + f" {fmt(r['recover_after_rise_s'], 1):>7} ({r['recover_censored'] or 0}/{r['rises']})".ljust(20)
                + f" {fmt(r['excess_area_p50']):>7} {fmt(r['excess_area_p90']):>7} {fmt(r['excess_peak_p50']):>6}"
                f" {fmt(r['flaps_per_min'], 2):>6}")
        return "\n".join(lines)

    emit(a, out, text)


def cmd_osc(a):
    v, have = views(a)
    need(have, "verdicts", "episodes")
    sql = """
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
SELECT * FROM m JOIN d USING (cell, arm) ORDER BY ALL;"""
    out = duck(v + sql)

    def text():
        lines = [f"{'cell':22} {'arm':24} {'runs':>4} {'flaps/min':>9} {'unhealthy spells':>16} "
                 f"{'dwell s: healthy p50 p90':>24} {'unhealthy p50 p90':>18} {'periodic':>8} {'period_s':>8} "
                 f"{'acf':>5} {'amp_ms p50 p90':>15}"]
        for r in out:
            lines.append(
                f"{r['cell'] or '-':22} {short(r['arm']) or '-':24} {r['runs']:4d} {fmt(r['flaps_per_min'], 2):>9} "
                f"{r['unhealthy_spells']:>16} {fmt(r['healthy_p50']):>15} {fmt(r['healthy_p90']):>8} "
                f"{fmt(r['unhealthy_p50']):>9} {fmt(r['unhealthy_p90']):>8} {r['periodic']:>8} "
                f"{fmt(r['period_s'], 1):>8} {fmt(r['acf'], 2):>5} {fmt(r['amp_ms']):>7} {fmt(r['amp_p90_ms']):>7}")
        return "\n".join(lines)

    emit(a, out, text)


def cmd_forecast(a):
    v, have = with_samples(a, *views(a, derived=False), ("forecast",), derived=False)
    need(have, "forecast")
    w = " AND ".join(["run = 0", "NOT first_sight", "mean_ms >= 0", *time_filter(a)])
    sql = f"""
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
       avg((held_ms < 0)::INT) FILTER (WHERE peer_honest) AS censored,
       avg(backlog::INT) FILTER (WHERE peer_honest) AS backlog
FROM f GROUP BY ALL ORDER BY ALL;"""
    out = duck(v + sql)

    def q4(x):
        return "-" if not x else " ".join(fmt(y) for y in x)

    def text():
        lines = [f"{'sim':10} {'cell':20} {'arm':24} {'pace':6} {'n':>8} {'err ms p10 p50 p90 p99':>23} "
                 f"{'cover':>5} {'held':>7} {'truth err p10 p50 p90 p99':>26} {'HAVE late p10 p50 p90 p99':>26} "
                 f"{'cens':>5} {'backlog':>7}"]
        for r in out:
            lines.append(
                f"{r['sim'] or '-':10} {r['cell'] or '-':20} {short(r['arm']) or '-':24} {r['pace']:6} {r['n']:8d} "
                f"{q4(r['err_q']):>23} {fmt(r['coverage'], 2):>5} {r['held']:7d} {q4(r['truth_q']):>26} "
                f"{q4(r['late_q']):>26} {fmt(r['censored'], 3):>5} {fmt(r['backlog'], 3):>7}")
        return "\n".join(lines)

    emit(a, out, text)


def cmd_rows(a):
    v, _ = views(a, derived=False)
    keys = duck(v + "\nSELECT src, k, cell, arm, seed FROM runs ORDER BY cell, arm, seed;")
    got = rows([(r["src"], r["k"]) for r in keys])
    want = a.fields.split(",") if a.fields else None
    for r in keys:
        row = got.get((r["src"], r["k"]))
        if row is None:
            continue
        v_ = row.pop("_verdict") or {}
        out = {"cell": r["cell"], "arm": r["arm"], "seed": r["seed"], "k": r["k"], "src": r["src"]}
        out.update({f: row.get(f) for f in want} if want else row)
        out["_verdict"] = {x: v_.get(x) for x in ("onset", "recovered", "absorbing", "unhealthy", "lag_p50")}
        out["_collapsed"] = analyze().collapsed(v_) if v_ else None
        print(json.dumps(out))


# --- cache --------------------------------------------------------------------


def farm_bucket():
    cfg = main_checkout() / "hack" / "sim-farm" / "config.toml"
    if not cfg.exists():
        sys.exit(f"no {cfg}: the farm's bucket is its s3_bucket")
    c = tomllib.loads(cfg.read_text())
    return c["s3_bucket"], c.get("s3_region"), c.get("aws_profile")


def aws(args, region, profile):
    """Runs the aws CLI's `args` in the farm bucket's region, as its
    profile."""
    return subprocess.run(["aws", *args, *(["--region", region] if region else []),
                           *(["--profile", profile] if profile else [])], capture_output=True, text=True)


# The aws CLI's multipart size: it downloads a file past it a ranged GET a
# part (farm.py's CLI_PART).
CLI_PART = 8 * 1024 * 1024


def pull(run, reads=False):
    """Copies farm run `run`'s flushes from S3 into the cache
    (farm/<run>/<instance>/<seq>/), and with `reads` their reads too. What
    it cost S3 goes to the run's line of the farm's S3 ledger
    (target/sim-farm/s3.jsonl, farm.py's S3_LEDGER): a LIST a thousand of a
    prefix's keys, a GET a file it downloaded (a part each past CLI_PART),
    and the bytes, out to the internet."""
    bucket, region, profile = farm_bucket()
    dest = cache_root() / "cache" / "farm" / run
    lists = gets = got = 0
    for prefix in ["sim-data"] + (["sim-samples"] if reads else []):
        p = aws(["s3", "sync", f"s3://{bucket}/{prefix}/{run}/", str(dest), "--no-progress"], region, profile)
        p.check_returncode()
        for line in p.stdout.splitlines():
            if line.startswith("download: ") and " to " in line:
                size = Path(line.rsplit(" to ", 1)[1]).stat().st_size
                gets += 1 if size < CLI_PART else math.ceil(size / CLI_PART)
                got += size
        keys = sum(1 for f in dest.glob("**/*")
                   if f.is_file() and (f.stem in SAMPLE_TABLES) == (prefix == "sim-samples"))
        lists += max(1, math.ceil(keys / 1000))
    farm_ledger(run, {"region": region or "", "tier1": {"pull_list": lists}, "tier2": {"pull_get": gets},
                      "up": {}, "down": {"internet": got}, "stored": 0})
    return dest


def farm_ledger(run, use):
    """A pull's line of the farm's S3 ledger, appended under its lock as
    farm.py appends (append_jsonl)."""
    path = main_checkout() / "target" / "sim-farm" / "s3.jsonl"
    path.parent.mkdir(parents=True, exist_ok=True)
    rec = {"run": run, "at": time.strftime("%Y-%m-%dT%H:%M:%S+00:00", time.gmtime()), "source": "pull", **use}
    with open(path, "ab+") as f:
        fcntl.flock(f, fcntl.LOCK_EX)
        end = f.seek(0, os.SEEK_END)
        torn = False
        if end:
            f.seek(end - 1)
            torn = f.read(1) != b"\n"
        f.write((("\n" if torn else "") + json.dumps(rec) + "\n").encode())
        f.flush()
        os.fsync(f.fileno())


def pull_sources(srcs):
    """Pulls the farm runs of the sources `srcs` (farm/<run>/...) that the
    cache lacks, a run at a time, less their samples (pull_samples);
    returns those still missing (expired from S3, or never uploaded)."""
    want = sorted({s for s in srcs if s.startswith("farm/") and not have_src(s)})
    for run in sorted({s.split("/")[1] for s in want}):
        pull(run)
    return [s for s in want if not have_src(s)]


def manifest_of(src):
    """Source `src`'s manifest in the cache (export's: its keys, and its
    tables' rows, bytes and sha256), or {}."""
    try:
        return json.loads((cache_root() / "cache" / src / "manifest.json").read_text())
    except (OSError, ValueError):
        return {}


def unlanded(srcs):
    """The farm flushes among `srcs` whose samples never landed in S3, as
    their runs' matrices' indexes say (a line's `reads` false; a run's
    matrices its state's list)."""
    farm = main_checkout() / "target" / "sim-farm"
    names = set()
    for run in {s.split("/")[1] for s in srcs if s.startswith("farm/")}:
        with contextlib.suppress(OSError):
            names |= set((farm / run / "matrices").read_text().split())
    out = set()
    for n in sorted(names):
        with contextlib.suppress(OSError):
            for line in (farm / n / "index.jsonl").read_text().splitlines():
                with contextlib.suppress(ValueError):
                    r = json.loads(line)
                    if isinstance(r, dict) and r.get("reads") is False and r.get("src") in srcs:
                        out.add(r["src"])
    return out


def sample_plan(v, tables, gone=()):
    """What the jobs the views `v` select lack of the sample `tables` that
    S3 holds: {src: (tables, bytes, jobs)}, a farm flush each, and how many
    jobs no flush in S3 gives them. A key's tables come from the source
    holding the most of them (views' canon), so a job lacks nothing once a
    source of it at the top holds what its manifest lists of `tables`; else
    one of those is fetched, and then holds the most. The flushes are the
    fewest that give every such job, picked greedily (the one giving the
    most of those left, the fewer bytes on a tie), so a verify copy's
    flush comes only for a job its original's doesn't give. A flush gives
    none once `gone` from S3, nor when its matrices' indexes say its
    samples never landed (unlanded)."""
    root = cache_root() / "cache"
    cands = collections.defaultdict(set)
    for r in duck(v + "\nSELECT DISTINCT k, src FROM runs_all SEMI JOIN runs USING (k);"):
        cands[r["k"]].add(r["src"])
    farm = {s for ss in cands.values() for s in ss if s.startswith("farm/")}
    if not farm:
        return {}, 0
    held = {s: sum((root / s / f"{t}.parquet").exists() for t in TABLES) for ss in cands.values() for s in ss}
    listed = {s: manifest_of(s).get("tables") or {} for s in farm}
    lack = {s: [t for t in tables if t in listed.get(s, {}) and not (root / s / f"{t}.parquet").exists()]
            for s in held}
    bad = unlanded(farm) | set(gone)
    need, lost = {}, 0
    for k, ss in cands.items():
        most = max(held[s] for s in ss)
        top = [s for s in ss if held[s] == most]
        if any(not lack[s] for s in top):
            continue
        if can := [s for s in top if s not in bad]:
            need[k] = can
        else:
            lost += 1
    size = {s: sum(listed[s][t].get("bytes", 0) for t in lack[s]) for s in farm}
    plan = {}
    while need:
        score = collections.Counter(s for ss in need.values() for s in ss)
        s = max(score, key=lambda s: (score[s], -size[s], s))
        plan[s] = (lack[s], size[s], score[s])
        need = {k: ss for k, ss in need.items() if s not in ss}
    return plan, lost


def fetch_samples(plan, gets, got):
    """Fetches `plan`'s tables (sample_plan's) from S3 into the cache, a
    GET a file (s3api get-object: one request whatever the size), each
    checked against its flush's manifest and put in place whole; returns
    the flushes S3 no longer holds. Counts each run's GETs (a missing
    key's too) in `gets` and their bytes in `got`, however it ends."""
    bucket, region, profile = farm_bucket()
    root = cache_root() / "cache"
    gone = set()
    for src, (tables, _, _) in plan.items():
        _, run, flush = src.split("/", 2)
        want = manifest_of(src)["tables"]
        for t in tables:
            dest = root / src / f"{t}.parquet"
            part = dest.with_name(f".{dest.name}.tmp{os.getpid()}")
            gets[run] += 1
            try:
                try:
                    p = aws(["s3api", "get-object", "--bucket", bucket, "--key",
                             f"sim-samples/{run}/{flush}/{t}.parquet", str(part)], region, profile)
                finally:
                    got[run] += part.stat().st_size if part.exists() else 0
                if p.returncode:
                    if "NoSuchKey" in p.stderr or "(404)" in p.stderr:
                        gone.add(src)
                        break
                    sys.exit(f"{src}: {t}: {p.stderr.strip()}")
                if part.stat().st_size != want[t]["bytes"] or sha256(part) != want[t]["sha256"]:
                    sys.exit(f"{src}: S3's {t}.parquet ({part.stat().st_size} bytes) isn't its manifest's")
                part.replace(dest)
            finally:
                part.unlink(missing_ok=True)
    return gone


def pull_samples(v, tables, limit_mb=None):
    """Pulls what the jobs the views `v` select lack of the sample `tables`
    (sample_plan), noting its size on stderr, and refusing it past
    `limit_mb`; where S3 no longer holds a flush, another copy of its jobs
    stands in. Each run's GETs and bytes go to its line of the farm's S3
    ledger, however the pull ends. Returns whether it fetched any."""
    plan, lost = sample_plan(v, tables)
    if plan:
        total = sum(b for _, b, _ in plan.values())
        what = (f"the {' and '.join(tables)} of {sum(n for *_, n in plan.values())} jobs the selection lacks: "
                f"{len(plan)} flushes, {total / 1e6:.1f} MB")
        if limit_mb is not None and total > limit_mb * 1e6:
            sys.exit(f"{what}, past --pull-mb {limit_mb:g}: narrow the selection, or raise --pull-mb")
        print(f"pulling {what}", file=sys.stderr)
    fetched, gone = bool(plan), set()
    gets, got = collections.Counter(), collections.Counter()
    try:
        while plan:
            gone |= fetch_samples(plan, gets, got)
            plan, lost = sample_plan(v, tables, gone)
    finally:
        region = farm_bucket()[1] if gets else None
        for run in gets:
            farm_ledger(run, {"region": region or "", "tier1": {}, "tier2": {"pull_get": gets[run]}, "up": {},
                              "down": {"internet": got[run]}, "stored": 0})
    if lost:
        print(f"S3 holds none of the {' or '.join(tables)} of {lost} of the selection's jobs (expired, or never "
              "landed): left out", file=sys.stderr)
    return fetched


def cmd_pull(a):
    picked = a.cell or a.arm or a.seed
    if (picked or a.dry_run) and not a.reads:
        sys.exit("--cell, --arm, --seed and --dry-run are of a pull's reads: give --reads")
    if not a.dry_run and not picked:
        dest = pull(a.run, a.reads)
        print(f"{a.run}: {len(sources([f'farm/{a.run}']))} sources in {dest}")
        return
    srcs = sources([f"farm/{a.run}"])
    if not srcs:
        if a.dry_run:
            sys.exit(f"the cache lacks farm/{a.run}: pull --run {a.run} copies the tables that name its jobs")
        pull(a.run)
    if picked:
        sel = argparse.Namespace(src=[f"farm/{a.run}"], cell=a.cell, arm=a.arm, seed=a.seed)
        v, _ = views(sel, derived=False, fetch=not a.dry_run)
        if not a.dry_run:
            pull_samples(v, SAMPLE_TABLES)
            print(f"{a.run}: the selection's {' and '.join(SAMPLE_TABLES)} in {cache_root() / 'cache' / 'farm'}")
            return
        plan, lost = sample_plan(v, SAMPLE_TABLES)
    else:
        # The whole run's, as `pull --reads` syncs them: each flush's.
        root, bad = cache_root() / "cache", unlanded(srcs)
        plan, lost = {}, 0
        for s in srcs:
            m = manifest_of(s)
            listed, jobs = m.get("tables") or {}, len(m.get("keys") or [])
            lack = [t for t in SAMPLE_TABLES if t in listed and not (root / s / f"{t}.parquet").exists()]
            if lack and s in bad:
                lost += jobs
            elif lack:
                plan[s] = (lack, sum(listed[t].get("bytes", 0) for t in lack), jobs)
    bucket, _, _ = farm_bucket()
    files = [(f"s3://{bucket}/sim-samples/{s.split('/', 1)[1]}/{t}.parquet", manifest_of(s)["tables"][t]["bytes"], n)
             for s, (tables, _, n) in plan.items() for t in tables]
    gone = f"; S3 holds none for {lost} jobs (expired, or never landed)" if lost else ""
    if not files:
        print(f"{a.run}: nothing to pull: the cache holds what S3 has of the jobs' samples{gone}")
        return
    print(f"{a.run}: would GET {len(files)} files, {sum(b for _, b, _ in files) / 1e6:.1f} MB, for "
          f"{sum(n for *_, n in plan.values())} jobs' {' and '.join(SAMPLE_TABLES)}{gone}")
    for url, b, n in files:
        print(f"  {url}  {b / 1e6:.1f} MB  {n} jobs")


def cmd_gc(a):
    root = cache_root()
    cutoff = time.time() - a.days * 86400
    gone = 0
    for runs in (root / "cache").glob("**/runs.parquet"):
        d = runs.parent
        if d.stat().st_mtime < cutoff:
            shutil.rmtree(d)
            gone += 1
    # An export a killed process left half written.
    for d in (root / "cache").glob("**/*.tmp*"):
        if d.is_dir() and d.stat().st_mtime < time.time() - 3600:
            shutil.rmtree(d)
    current = f"v={analyze().VERDICT_VERSION}"
    for d in (root / "derived").glob("*/v=*"):
        if d.name != current:
            shutil.rmtree(d)
    live = {s.replace("/", "~") for s in sources(None)}
    for f in (root / "derived").glob("*/v=*/*.parquet"):
        if f.stem not in live:
            f.unlink()
    print(f"removed {gone} sources older than {a.days} days")


def main():
    raw = argparse.RawDescriptionHelpFormatter
    ap = argparse.ArgumentParser(description=__doc__, epilog=SQL_DOC, formatter_class=raw)
    sub = ap.add_subparsers(dest="cmd", required=True)
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
    p.add_argument("--reads", action="store_true", help="the reads and forecast logs too (sim-samples/)")
    p.add_argument("--cell", action="append", help="only these cells' reads, globs")
    p.add_argument("--arm", action="append", help="only these arms' reads, globs")
    p.add_argument("--seed", action="append", type=int, help="only these seeds' reads")
    p.add_argument("--dry-run", action="store_true", help="list the reads' files a pull would fetch, asking S3 nothing")
    g = sub.add_parser("gc")
    g.add_argument("--days", type=float, default=7)
    qs = {}
    qs["sql"] = sub.add_parser("sql", help="a query over the views", description=SQL_DOC, formatter_class=raw)
    qs["sql"].add_argument("query")
    qs["sql"].add_argument("--reads", action="store_true",
                           help="pull the selection's reads and forecast logs first, where the cache lacks them")
    qs["pooled"] = sub.add_parser("pooled")
    qs["pooled"].add_argument("--by", choices=("none", "phase", "bin", "viewer"), default="none")
    qs["aligned"] = sub.add_parser("aligned")
    qs["aligned"].add_argument("event", help=f"a derived event ({', '.join(DERIVED_KINDS)}) or a sim's (its "
                               "events table's kinds: join, leave, first_read, first_frame, ...; join and leave "
                               "a node's)")
    qs["aligned"].add_argument("--node", type=int, help="only events of this node (the sims' events)")
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
    else:
        {"sql": cmd_sql, "pooled": cmd_pooled, "aligned": cmd_aligned, "events": cmd_events, "osc": cmd_osc,
         "rows": cmd_rows, "forecast": cmd_forecast}[a.cmd](a)


if __name__ == "__main__":
    main()
