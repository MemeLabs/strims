# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The store (store_host's) run in the module, the cache it exports into,
and pulls from the farm's S3.

The store's fold and export (store_host's, which simdb.py's names are)
build their SQL scripts and run them through store_host.duck, which runs
the DuckDB CLI unless an executor is installed (store_host.EXECUTOR): the
farm hosts, which ship store_host alone, run the CLI. `install` installs
this module's `execute`, so a script running under the engine
(simlab.bootstrap.engine) folds, exports and queries in the module, over
the same SQL.

  execute(script, db=None, out="json", threads=None, memory=None)
                 store_host.duck's contract, in the module: `script` run over
                 the DuckDB file `db` (made if missing), else in memory,
                 statement by statement as the CLI runs them (stopping at
                 the first that fails: a transaction it opened rolls
                 back), with duck's settings (threads, memory_limit,
                 the cache's tmp/ to spill to); returns the last shown
                 statement's rows as dicts of the CLI's JSON values
                 (engine.Result.records; [] for none), the CLI's -table
                 text of every shown statement (`out` "text"), or None
                 (`out` None); store_host.DuckError, DuckDB's message,
                 on a failure
  results(conn, script, out="json")
                 the same over the connection `conn`
  shown(conn, script, text=False) -> [engine.Result]
                 the Results of the statements of `script` the CLI prints,
                 over `conn`
  spill()        the cache's tmp/, which the module's scripts spill to
  executor(conn) duck's contract over `conn`, an instance in memory a
                 process holds (a derive worker's): its scripts run on it,
                 one over a DuckDB file in a connection of its own
  install()      store_host.EXECUTOR = execute
  installed()    a block whose duck runs in the module where the duckdb
                 module imports (else in the CLI), the executor before it
                 back after: a simlab.suite invocation's store

The cache (store_host.cache_root's cache/) holds an export a source:
suite/<worktree>/<invocation>/ for a local suite run's, farm/<run>/
<instance>/<seq>/ for a farm flush's, each its tables' Parquet files and
manifest.json (store_host.export's):

  sources(patterns) -> [src]
                 the cache's sources matching `patterns` (globs; every one
                 by default)
  have_src(src)  whether the cache holds source `src`
  manifest_of(src) -> dict
                 its manifest, or {}
  gc(days) -> int
                 the sources older than `days` removed (how many), with the
                 derived tables of no current source or version

A farm run's flushes are in the farm's bucket (hack/sim-farm/config.toml's
s3_bucket, s3_region and aws_profile), each under
sim-data/<run>/<instance>/<seq>/ and, its bulky tables (store_host's
SAMPLE_TABLES), sim-samples/<run>/<instance>/<seq>/. A run is named for
its matrices (farm.py's run_name: their names joined with "+", past EC2's
tag limit the first's, "+<n>more-" and a hash), and a suite farm run's
matrices for its id (suite.py's write_matrices: <id>-NN):

  farm_runs() -> {run: [matrix]}
                 the runs the farm's state lists the matrices of
                 (target/sim-farm/<run>/matrices), and those the cache
                 holds flushes of, their matrices the ones their names join
  farm_run(name, runs=None) -> Named
                 the run `name` names: the run of that name, else the one
                 run including a matrix of that name or a suite run's
                 matrices of that id, else `name` itself, unknown (S3 may
                 hold it all the same); of a run of that name (a matrix
                 run alone) and those that include it, the one started
                 in the S3_KEPT_DAYS days S3 keeps a run (started) if one
                 alone was, else the run of that name, naming the others;
                 Refused, naming them, where several runs include a name
                 no run has
  started(run) -> float | None
                 when a run started: its state's matrices list's time,
                 else its newest flush's in the cache
  unknown(name, s3=False, runs=None) -> str
                 that no run the state or the cache knows is or includes
                 `name` (nor, with `s3`, does S3 hold one of that name),
                 with the names nearest it
  unpulled(named, runs=None) -> str
                 why a pull of `named` found nothing: S3 holds nothing
                 under the run's sim-data/, and the cache none of it (for
                 an unknown name, unknown's)
  pull(run, reads=False) -> Pull
                 `aws s3 sync` of the run's sim-data (and sim-samples with
                 `reads`) into the cache's farm/<run>/, its LISTs, GETs
                 and bytes a line of the farm's S3 ledger; a sync that
                 wrote nothing leaves no directory
  pull_sources(srcs) -> ([Pull], gone)
                 the pulls of the farm runs of the sources `srcs` the
                 cache lacks, less their samples, and the sources still
                 missing after them (expired from S3, or never uploaded)
  unlanded(srcs) -> set
                 the farm flushes among `srcs` whose samples never landed
                 in S3, as their matrices' indexes say
  sample_plan(views, tables, gone=()) -> ({src: (tables, bytes, jobs)}, lost)
                 what the jobs a selection picks lack of the sample
                 `tables` that S3 holds, the fewest flushes that give it,
                 and how many jobs no flush gives; `views` is SQL making
                 the selection's `runs_all` and `runs` (simlab.query's
                 Views.sql)
  listed_samples(srcs, tables=SAMPLE_TABLES) -> [table]
                 those of `tables` some of the flushes `srcs` hold (their
                 manifests list), which a pull's lines name
  pull_samples(views, tables, limit_mb=None, say=None) -> bool
                 the plan of `tables` and their truth (with_truth) fetched,
                 noting its size, refused (Refused) past `limit_mb`;
                 whether it fetched any
  with_truth(tables) -> tuple
                 `tables` and the tables their views take their truth
                 from (TRUTH)
  fetch_samples(plan, gets, got) -> set
                 the tables of sample_plan's `plan`, a GET a file, each
                 checked against its flush's manifest (size and sha256) and
                 put in place whole; returns the flushes S3 no longer
                 holds, its GETs and their bytes counted by run in `gets`
                 and `got` (Counters) for the caller's ledger line, however
                 it ends
  farm_ledger(run, use)
                 appends a pull's line to the farm's S3 ledger
                 (target/sim-farm/s3.jsonl, farm.py's S3_LEDGER) under its
                 lock, as farm.py appends
  farm_bucket() -> (bucket, region, profile)
  aws(args, region, profile)
                 the aws CLI's `args` in the bucket's region, as its
                 profile

Refused is an operation refused, its message why: uncaught, it ends a
script as sys.exit(message) does, and a caller that serves several (the
console's server) catches it.
"""

import collections
import contextlib
import dataclasses
import difflib
import fcntl
import json
import math
import os
import re
import shutil
import subprocess
import sys
import time
import tomllib
from pathlib import Path

from . import engine, store_host, verdict


class Refused(SystemExit):
    """An operation refused, its message why (the module docs)."""


# The tables whose views take their truth from others' (simlab.query's
# asks_view, haves_view and seed_sends_view: the peers' holds in arrivals),
# which a pull of them brings along: the sample tables asks and haves, and
# seed_sends, which comes with sim-data while its truth is a sample table.
TRUTH = {"asks": ("arrivals",), "haves": ("arrivals",), "seed_sends": ("arrivals",)}


def with_truth(tables):
    """`tables`, then the tables their views take their truth from (TRUTH)
    that `tables` lacks."""
    return tuple(dict.fromkeys([*tables, *(x for t in tables for x in TRUTH.get(t, ()))]))


@dataclasses.dataclass
class Pull:
    """What a pull copied: the run's directory in the cache (`dest`), the
    flushes it wrote files into (sources, farm/<run>/<instance>/<seq>),
    and its line of the farm's S3 ledger (`use`: its region, requests by
    tier and bytes)."""
    run: str
    dest: Path
    pulled: list = dataclasses.field(default_factory=list)
    use: dict = dataclasses.field(default_factory=dict)


def stderr(message):
    print(message, file=sys.stderr)


# --- the executor ---------------------------------------------------------------------


def execute(script, db=None, out="json", threads=None, memory=None):
    """Runs `script` as store_host.duck would in the CLI (the module
    docs)."""
    engine.need_module()
    try:
        conn = engine.store(db or ":memory:", threads, memory, spill())
    except engine.duckdb.Error as e:
        raise store_host.DuckError(str(e)) from None
    try:
        return results(conn, script, out)
    finally:
        conn.close()


def spill():
    """The cache's tmp/ (made), which duck's scripts spill to; None while
    the cache's root doesn't exist."""
    tmp = store_host.cache_root() / "tmp"
    if not tmp.parent.exists():
        return None
    tmp.mkdir(exist_ok=True)
    return tmp


def shown(conn, script, text=False):
    """Runs `script` on `conn` statement by statement (engine.script), and
    returns the Results of those the CLI prints, with `text` their values
    DuckDB's strings; store_host.DuckError, DuckDB's message, at the first
    that fails."""
    try:
        return engine.script(conn, script, text=text)
    except (engine.QueryError, engine.duckdb.Error) as e:
        raise store_host.DuckError(str(e)) from None


def results(conn, script, out="json"):
    """Runs `script` on `conn` statement by statement, and returns what
    store_host.duck returns for `out`; store_host.DuckError, DuckDB's
    message, at the first that fails."""
    got = shown(conn, script, text=out == "text")
    if out == "json":
        return got[-1].records() if got else []
    if out == "text":
        return "".join(r.table() for r in got)
    return None


def executor(conn):
    """store_host.duck's contract over `conn`: a script with no `db` runs
    on it (its threads and memory as `conn` has them), one over a DuckDB
    file in a connection of its own (execute)."""
    def run(script, db=None, out="json", threads=None, memory=None):
        if db is not None:
            return execute(script, db, out, threads, memory)
        return results(conn, script, out)
    return run


def install():
    """Makes duck run its scripts in the module (`execute`)."""
    store_host.EXECUTOR = execute


@contextlib.contextmanager
def installed():
    """duck runs its scripts in the module inside the block, where the
    duckdb module imports (a script under the engine), else in the CLI;
    the executor before it is back after."""
    before = store_host.EXECUTOR
    if engine.duckdb is not None:
        store_host.EXECUTOR = execute
    try:
        yield
    finally:
        store_host.EXECUTOR = before


# --- the cache ------------------------------------------------------------------------


def sources(patterns):
    """The cache's sources (dirs holding runs.parquet) matching `patterns`."""
    root = store_host.cache_root() / "cache"
    out = set()
    for p in patterns or ["**"]:
        for m in root.glob(p):
            if (m / "runs.parquet").exists():
                out.add(str(m.relative_to(root)))
            for r in m.glob("**/runs.parquet"):
                out.add(str(r.parent.relative_to(root)))
    return sorted(out)


def have_src(src):
    """Whether the cache holds source `src`'s export."""
    return (store_host.cache_root() / "cache" / src / "runs.parquet").exists()


def manifest_of(src):
    """Source `src`'s manifest in the cache (export's: its keys, and its
    tables' rows, bytes and sha256), or {}."""
    try:
        return json.loads((store_host.cache_root() / "cache" / src / "manifest.json").read_text())
    except (OSError, ValueError):
        return {}


def gc(days):
    """Removes the cache's sources older than `days`, the exports a killed
    process left half written (an hour on), the derived tables of other
    derived_versions and those of sources the cache no longer holds;
    returns how many sources it removed."""
    root = store_host.cache_root()
    cutoff = time.time() - days * 86400
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
    current = f"v={verdict.derived_version()}"
    for d in (root / "derived").glob("*/v=*"):
        if d.name != current:
            shutil.rmtree(d)
    live = {s.replace("/", "~") for s in sources(None)}
    for f in (root / "derived").glob("*/v=*/*.parquet"):
        if f.stem not in live:
            f.unlink()
    return gone


# --- S3 -------------------------------------------------------------------------------

# The aws CLI's multipart size: it downloads a file past it a ranged GET a
# part (farm.py's CLI_PART).
CLI_PART = 8 * 1024 * 1024


def farm_bucket():
    cfg = store_host.main_checkout() / "hack" / "sim-farm" / "config.toml"
    if not cfg.exists():
        sys.exit(f"no {cfg}: the farm's bucket is its s3_bucket")
    c = tomllib.loads(cfg.read_text())
    return c["s3_bucket"], c.get("s3_region"), c.get("aws_profile")


def aws(args, region, profile):
    """Runs the aws CLI's `args` in the farm bucket's region, as its
    profile."""
    return subprocess.run(["aws", *args, *(["--region", region] if region else []),
                           *(["--profile", profile] if profile else [])], capture_output=True, text=True)


def farm_state():
    """The farm's state: the main checkout's target/sim-farm (farm.py's
    FARM)."""
    return store_host.main_checkout() / "target" / "sim-farm"


# farm.py beside simlab, whose constants the lab reads (farm_constants).
FARM_PY = Path(__file__).resolve().parents[1] / "sim-farm" / "farm.py"


def farm_constants(names, path=FARM_PY):
    """{name: value} of those of `names` that farm.py (`path`) defines at
    its top level as arithmetic of literals (`S3_KEPT_DAYS = 7`), read from
    its source, as floats; none where it can't be read."""
    out = {}
    try:
        src = Path(path).read_text()
    except OSError:
        return out
    for name in names:
        m = re.search(rf"^{name} = (.+)$", src, re.M)
        if m and re.fullmatch(r"[0-9.e+\-*/ <()]+", m.group(1).strip()):
            out[name] = float(eval(m.group(1), {"__builtins__": {}}))
    return out


# How long S3 keeps a run's data, in days: farm.py's S3_KEPT_DAYS.
S3_KEPT_DAYS = int(farm_constants(["S3_KEPT_DAYS"])["S3_KEPT_DAYS"])
# The part of a run's name standing for the matrices past EC2's tag limit
# (farm.py's run_name).
MORE = re.compile(r"\d+more-\w+")


def farm_runs():
    """{run: [matrix]}: the runs whose state lists their matrices
    (farm_state()/<run>/matrices, which a run writes as it starts), and
    those the cache holds flushes of (farm/<run>/<instance>/<seq>/) that
    the state doesn't list, their matrices those their names join."""
    runs = {}
    with contextlib.suppress(OSError), os.scandir(farm_state()) as it:
        for e in it:
            with contextlib.suppress(OSError):
                runs[e.name] = (Path(e.path) / "matrices").read_text().split()
    with contextlib.suppress(OSError), os.scandir(store_host.cache_root() / "cache" / "farm") as it:
        for e in it:
            if e.name not in runs and next(Path(e.path).glob("*/*/runs.parquet"), None):
                runs[e.name] = [m for m in e.name.split("+") if not MORE.fullmatch(m)]
    return runs


def suite_id(matrix):
    """The id of the suite farm run that matrix `matrix` is one of
    (suite.py's write_matrices: <id>-NN), else None."""
    stem, _, n = matrix.rpartition("-")
    return stem if stem and len(n) >= 2 and n.isdigit() else None


def includes(name, matrices):
    """Those of `matrices` that `name` names, sorted: the matrix of that
    name, or the matrices of the suite farm run of that id."""
    return sorted(m for m in matrices if name in (m, suite_id(m)))


def span(matrices):
    """Matrices `matrices` (includes'), as the lines name them."""
    if len(matrices) == 1:
        return f"matrix {matrices[0]}"
    return f"matrices {matrices[0]} to {matrices[-1]} ({len(matrices)})"


@dataclasses.dataclass
class Named:
    """The farm run `run` that the name `asked` names (farm_run); the
    run's `matrices` it names, where it names matrices rather than the
    run; whether the farm's state or the cache knows the run (`known`);
    the other runs the name names too (`also`: a run of that name, or
    those that include it)."""
    run: str
    asked: str
    matrices: list = dataclasses.field(default_factory=list)
    known: bool = True
    also: list = dataclasses.field(default_factory=list)

    def text(self):
        """The line saying which run the name names, where it names
        matrices or other runs too; else None."""
        if not self.matrices and not self.also:
            return None
        what = f"farm run {self.run}"
        if self.matrices:
            what = f"{'a matrix' if self.matrices == [self.asked] else span(self.matrices)} of {what}"
        if self.also:
            others = ", ".join(f"farm run {r} (started {when(started(r))})" for r in self.also)
            what += (f", the one run it names started in the {S3_KEPT_DAYS} days S3 keeps a run's data; it also "
                     f"names {others}" if self.matrices else
                     f"; it also names {others}, which {'include' if len(self.also) > 1 else 'includes'} it: pull "
                     "--run one by its name for it")
        return f"{self.asked}: {what}"


def started(run):
    """When farm run `run` started, in seconds since the epoch: its state's
    matrices list's modification time (a run writes it as it starts),
    else the newest of its flushes' in the cache; None for neither."""
    with contextlib.suppress(OSError):
        return (farm_state() / run / "matrices").stat().st_mtime
    times = []
    with contextlib.suppress(OSError):
        for f in (store_host.cache_root() / "cache" / "farm" / run).glob("*/*/runs.parquet"):
            with contextlib.suppress(OSError):
                times.append(f.stat().st_mtime)
    return max(times, default=None)


def when(t):
    """A time from started, as a date and time (UTC), or "when unknown"."""
    return "when unknown" if t is None else time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime(t))


def farm_run(name, runs=None):
    """The farm run `name` names, among `runs` (farm_runs' by default): the
    run of that name, else the one run that includes the matrix of that
    name or the matrices of the suite farm run of that id, else `name`
    itself, which S3 may hold all the same (unknown). Where a run of that
    name (a matrix run alone) and others that include it are among
    `runs`, the one of them started in the S3_KEPT_DAYS days S3 keeps a
    run's data, if one alone was, else the run of that name; the result
    names the others. Refused, naming them, where several runs include a
    name no run has."""
    runs = farm_runs() if runs is None else runs
    hits = {r: got for r, ms in sorted(runs.items()) if r != name and (got := includes(name, ms))}
    if name in runs:
        kept = time.time() - S3_KEPT_DAYS * 86400
        live = [r for r in (name, *hits) if (started(r) or 0) >= kept]
        if len(live) == 1 and live[0] != name:
            return Named(live[0], name, hits[live[0]], also=[name, *(r for r in hits if r != live[0])])
        return Named(name, name, also=list(hits))
    if len(hits) > 1:
        raise Refused(f"{name}: {len(hits)} farm runs include it; pull --run one of them:\n"
                      + "\n".join(f"  {r}: {span(ms)}" for r, ms in hits.items()))
    if hits:
        ((run, ms),) = hits.items()
        return Named(run, name, ms)
    return Named(name, name, known=False)


def nearest(name, runs, n=3):
    """The `n` names nearest `name` (difflib's, nearest first) among the
    names of `runs` (farm_runs'), of their matrices and of the suite farm
    runs those are of, each with the runs it names: [(name, [run])]."""
    names = collections.defaultdict(set)
    for r, ms in runs.items():
        names[r].add(r)
        for m in ms:
            names[m].add(r)
            if suite_id(m):
                names[suite_id(m)].add(r)
    return [(x, sorted(names[x])) for x in difflib.get_close_matches(name, list(names), n, 0.6)]


def unknown(name, s3=False, runs=None):
    """That no run the farm's state or the cache knows (`runs`, farm_runs'
    by default) is or includes `name`, nor (with `s3`) does S3 hold a run
    of that name, with the names nearest it."""
    near = ", ".join(x if rs == [x] else f"{x} (in {', '.join(rs)})"
                     for x, rs in nearest(name, farm_runs() if runs is None else runs))
    return (f"{name}: no farm run is or includes it: "
            + (f"S3 holds nothing under sim-data/{name}/, and " if s3 else "")
            + f"neither {farm_state()} nor the cache knows a run or matrix of that name, or matrices {name}-NN of a "
            "suite farm run" + (f"; the nearest: {near}" if near else ""))


def unpulled(named, runs=None):
    """Why a pull of `named` (farm_run's) found nothing: S3 holds nothing
    under its run's sim-data/<run>/ (it expired, or never landed), and the
    cache none of it; for a name of no run the farm's state or the cache
    knows, unknown's."""
    if not named.known:
        return unknown(named.asked, True, runs)
    head = f"{named.text()}; " if named.text() else f"{named.run}: "
    return (head + f"S3 holds nothing under sim-data/{named.run}/ (expired, or never landed), and the cache none of "
            "its flushes")


def pull(run, reads=False):
    """Copies farm run `run`'s flushes from S3 into the cache
    (farm/<run>/<instance>/<seq>/), and with `reads` their samples too.
    What it cost S3 goes to the run's line of the farm's S3 ledger: a LIST
    a thousand of a prefix's keys, a GET a file it downloaded (a part each
    past CLI_PART), and the bytes, out to the internet."""
    bucket, region, profile = farm_bucket()
    cache = store_host.cache_root() / "cache"
    dest = cache / "farm" / run
    lists = gets = got = 0
    pulled = set()
    for prefix in ["sim-data"] + (["sim-samples"] if reads else []):
        p = aws(["s3", "sync", f"s3://{bucket}/{prefix}/{run}/", str(dest), "--no-progress"], region, profile)
        p.check_returncode()
        for line in p.stdout.splitlines():
            if line.startswith("download: ") and " to " in line:
                # The CLI names the file relative to its working directory,
                # this process's.
                to = Path(os.path.abspath(line.rsplit(" to ", 1)[1]))
                size = to.stat().st_size
                gets += 1 if size < CLI_PART else math.ceil(size / CLI_PART)
                got += size
                pulled.add(to.parent.relative_to(os.path.abspath(cache)).as_posix())
        keys = sum(1 for f in dest.glob("**/*")
                   if f.is_file() and (f.stem in store_host.SAMPLE_TABLES) == (prefix == "sim-samples"))
        lists += max(1, math.ceil(keys / 1000))
    # The CLI makes `dest` for a prefix S3 holds nothing under.
    if dest.is_dir() and not any(dest.iterdir()):
        dest.rmdir()
    use = {"region": region or "", "tier1": {"pull_list": lists}, "tier2": {"pull_get": gets}, "up": {},
           "down": {"internet": got}, "stored": 0}
    farm_ledger(run, use)
    return Pull(run, dest, sorted(pulled), use)


def pull_sources(srcs):
    """Pulls the farm runs of the sources `srcs` (farm/<run>/...) that the
    cache lacks, a run at a time, less their samples (pull_samples);
    returns their Pulls and the sources still missing (expired from S3,
    or never uploaded)."""
    want = sorted({s for s in srcs if s.startswith("farm/") and not have_src(s)})
    pulls = [pull(run) for run in sorted({s.split("/")[1] for s in want})]
    return pulls, [s for s in want if not have_src(s)]


def farm_ledger(run, use):
    """A pull's line of the farm's S3 ledger, appended under its lock as
    farm.py appends (append_jsonl)."""
    path = farm_state() / "s3.jsonl"
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


def unlanded(srcs):
    """The farm flushes among `srcs` whose samples never landed in S3, as
    their runs' matrices' indexes say (a line's `reads` false; a run's
    matrices its state's list)."""
    farm = farm_state()
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


def candidates(views):
    """{k: {src}}: the sources holding each job the views `views` select."""
    cands = collections.defaultdict(set)
    for r in execute(views + "\nSELECT DISTINCT k, src FROM runs_all SEMI JOIN runs USING (k);"):
        cands[r["k"]].add(r["src"])
    return cands


def listed_samples(srcs, tables=store_host.SAMPLE_TABLES):
    """Those of the sample `tables` that some of the farm flushes `srcs`
    hold, as their manifests list them, in `tables`' order: what a pull of
    them can fetch, which its lines name."""
    listed = set()
    for s in srcs:
        if s.startswith("farm/"):
            listed |= set(manifest_of(s).get("tables") or {})
    return [t for t in tables if t in listed]


def sample_plan(views, tables, gone=(), cands=None):
    """What the jobs the views `views` select lack of the sample `tables`
    that S3 holds: {src: (tables, bytes, jobs)}, a farm flush each, and how
    many jobs no flush in S3 gives them. A key's tables come from the
    source holding the most of them (simlab.query's canon), so a job lacks
    nothing once a source of it at the top holds what its manifest lists
    of `tables`; else one of those is fetched, and then holds the most.
    The flushes are the fewest that give every such job, picked greedily
    (the one giving the most of those left, the fewer bytes on a tie), so
    a verify copy's flush comes only for a job its original's doesn't
    give. A flush gives none once `gone` from S3, nor when its matrices'
    indexes say its samples never landed (unlanded). `cands` are the
    views' candidates, when the caller has them."""
    root = store_host.cache_root() / "cache"
    cands = candidates(views) if cands is None else cands
    farm = {s for ss in cands.values() for s in ss if s.startswith("farm/")}
    if not farm:
        return {}, 0
    held = {s: sum((root / s / f"{t}.parquet").exists() for t in store_host.TABLES)
            for ss in cands.values() for s in ss}
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
    root = store_host.cache_root() / "cache"
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
                if part.stat().st_size != want[t]["bytes"] or store_host.sha256(part) != want[t]["sha256"]:
                    sys.exit(f"{src}: S3's {t}.parquet ({part.stat().st_size} bytes) isn't its manifest's")
                part.replace(dest)
            finally:
                part.unlink(missing_ok=True)
    return gone


def pull_samples(views, tables, limit_mb=None, say=None):
    """Pulls what the jobs the views `views` select lack of the sample
    `tables` (sample_plan), noting its size (on stderr, or to `say`), and
    refusing it past `limit_mb`; where S3 no longer holds a flush, another
    copy of its jobs stands in. Each run's GETs and bytes go to its line
    of the farm's S3 ledger, however the pull ends. Returns whether it
    fetched any. Its lines name the tables the jobs' flushes hold
    (listed_samples). `tables` bring their truth (with_truth)."""
    say = say or stderr
    tables = with_truth(tables)
    cands = candidates(views)
    named = listed_samples({s for ss in cands.values() for s in ss}, tables) or list(tables)
    plan, lost = sample_plan(views, tables, cands=cands)
    if plan:
        total = sum(b for _, b, _ in plan.values())
        what = (f"the {' and '.join(named)} of {sum(n for *_, n in plan.values())} jobs the selection lacks: "
                f"{len(plan)} flushes, {total / 1e6:.1f} MB")
        if limit_mb is not None and total > limit_mb * 1e6:
            raise Refused(f"{what}, past --pull-mb {limit_mb:g}: narrow the selection, or raise --pull-mb")
        say(f"pulling {what}")
    fetched, gone = bool(plan), set()
    gets, got = collections.Counter(), collections.Counter()
    try:
        while plan:
            gone |= fetch_samples(plan, gets, got)
            plan, lost = sample_plan(views, tables, gone, cands)
    finally:
        region = farm_bucket()[1] if gets else None
        for run in gets:
            farm_ledger(run, {"region": region or "", "tier1": {}, "tier2": {"pull_get": gets[run]},
                              "up": {}, "down": {"internet": got[run]}, "stored": 0})
    if lost:
        say(f"S3 holds none of the {' or '.join(named)} of {lost} of the selection's jobs (expired, or never "
            "landed): left out")
    return fetched
