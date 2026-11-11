# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The sims' store: jobs' tables folded into a DuckDB file and exported to
zstd Parquet, in the pinned DuckDB CLI. Stdlib alone, so the farm hosts
run it beside the worker (farm.py's HOST_FILES; worker.py's store()), and
hack/sim-db/simdb.py, whose `fold` and `export` commands and queries over
the exports it is, imports its names.

  duckdb_bin(fetch=True)
                 the pinned CLI (DUCKDB_VERSION, its sha256s), fetched
                 once into ~/.cache/strims/duckdb/; `SIM_DUCKDB` when set
  duck(script, db=None, out="json", threads=None, memory=None)
                 a SQL script run in the CLI (or in EXECUTOR, when one is
                 installed: simlab.store's, in the duckdb module), over
                 the file `db` or in memory: its last query's rows, its
                 text, or None; DuckError on a failure
  fold(db, jobs, src, threads=2, memory="1GB") -> {k: [table, ...]}
                 jobs' output dirs added to the store `db`, one
                 transaction a batch (FoldError for a job whose binary
                 wrote no tables)
  export(db, dest, threads=None) -> manifest
                 the store `db` written to `dest`, a Parquet file a table
                 and manifest.json
  recover(store_dir, dest_of)
                 the stores no process holds, exported and removed
  cache_root()   the cache's root: `SIM_DB_ROOT`, else the main
                 checkout's target/sim-db (its tmp/ is the CLI's spill)

TABLES are the sims' tables a job's dir may hold (`<table>.csv`, or the
large ones zstd-compressed as `<table>.csv.zst`, beside
`<table>.schema.json`), SAMPLE_TABLES the bulky ones the farm uploads
apart, and RUN_COLS and SCALAR_COLS the store's own `runs` and `scalars`
(simdb.py's docs).
"""

import contextlib
import fcntl
import functools
import gzip
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# The pinned DuckDB CLI: its release, and the sha256s of its download and
# of the binary. hack/pyproject.toml pins the duckdb module to the same
# release (simlab.engine checks it).
DUCKDB_VERSION = "v1.5.6"
DUCKDB_URL = f"https://github.com/duckdb/duckdb/releases/download/{DUCKDB_VERSION}/duckdb_cli-linux-amd64.gz"
DUCKDB_GZ_SHA256 = "cce2687a38681db644c248198b99ecd86170ff47e324c5f33e50c0042f142839"
DUCKDB_SHA256 = "61238cfbe9dfeaad4bcfcd8a48f6f7123dae2e9603aaadb1f77a4217aeb8980c"

# The sims' tables, and the store's own.
TABLES = ("seconds", "viewer_seconds", "reads", "stalls", "bins", "viewers", "events", "parents", "node_bins",
          "forecast", "spec_requests", "parks", "arrivals", "asks", "haves")
# The bulky tables: the farm uploads them under sim-samples/, and a pull
# fetches them only when asked (`--reads`, or a query that reads them).
SAMPLE_TABLES = ("reads", "forecast", "arrivals", "asks", "haves")
RUN_COLS = {
    "k": "VARCHAR", "src": "VARCHAR", "sim": "VARCHAR", "test": "VARCHAR", "cell": "VARCHAR", "arm": "VARCHAR",
    "seed": "BIGINT", "env": "VARCHAR", "trace": "VARCHAR", "frames": "BIGINT", "bytes": "BIGINT",
    "wall_s": "DOUBLE", "rss_mb": "DOUBLE", "at": "VARCHAR", "bin_ms": "BIGINT", "wrote": "VARCHAR",
    "meta": "VARCHAR", "row_json": "VARCHAR",
}
SCALAR_COLS = {"k": "VARCHAR", "src": "VARCHAR", "name": "VARCHAR", "value": "DOUBLE"}
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


# What runs duck()'s scripts in place of the CLI, to its contract (its
# arguments, return and DuckError): simlab.store's in the duckdb module,
# once installed; None, the CLI.
EXECUTOR = None


def duck(script, db=None, out="json", threads=None, memory=None):
    """Runs a SQL script in the DuckDB CLI, over `db` (a file) or in
    memory, and returns its last statement's rows (`out` "json"), its text
    (`out` "text": DuckDB's tables), or nothing (None); in EXECUTOR when
    one is installed.

    The CLI prints the values of some integer types (HUGEINT, which sum()
    of integers gives, UBIGINT, UHUGEINT) and of DECIMAL as JSON strings,
    to keep their precision. Rows come back with them as numbers: before
    a last statement that is a query (QUERY_WORDS) the script DESCRIBEs
    it, and its columns' types say which strings are numbers. A script
    the DESCRIBE fails runs as it is, its rows as the CLI prints them."""
    if EXECUTOR is not None:
        return EXECUTOR(script, db=db, out=out, threads=threads, memory=memory)
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


def table_csv(d, t):
    """Job dir `d`'s CSV of table `t`: `<t>.csv`, or the zstd stream
    `<t>.csv.zst` the sims write their large tables to (DuckDB's read_csv
    reads either); None without one or its schema."""
    if not (d / f"{t}.schema.json").exists():
        return None
    return next((p for p in (d / f"{t}.csv", d / f"{t}.csv.zst") if p.exists()), None)


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
            have = [t for t in TABLES if table_csv(d, t)]
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
                           f"read_csv({q(table_csv(d, t))}, header=true, auto_detect=false, columns={struct(c)});")
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
