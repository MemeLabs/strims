# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's store, run in the module, and pulls from the farm's S3.

simdb.py's fold and export build their SQL scripts and run them through
simdb.duck, which runs the DuckDB CLI unless an executor is installed
(simdb.EXECUTOR): the farm hosts, which ship the store alone, run the CLI.
`install` installs this module's `execute`, so a script running under the
engine (simlab.bootstrap.engine) folds, exports and queries in the module,
over the same SQL.

  execute(script, db=None, out="json", threads=None, memory=None)
                 simdb.duck's contract, in the module: `script` run over
                 the DuckDB file `db` (made if missing), else in memory,
                 statement by statement as the CLI runs them (stopping at
                 the first that fails: a transaction it opened rolls
                 back), with simdb.duck's settings (threads, memory_limit,
                 the cache's tmp/ to spill to); returns the last shown
                 statement's rows as dicts of the CLI's JSON values
                 (engine.Result.records; [] for none), the CLI's -table
                 text of every shown statement (`out` "text"), or None
                 (`out` None); simdb.DuckError, DuckDB's message, on a
                 failure
  install()      simdb.EXECUTOR = execute, on simlab.simdb()'s module
  installed()    a block whose duck runs in the module where the duckdb
                 module imports (else in the CLI), the executor before it
                 back after: a simlab.suite invocation's store

A farm run's flushes are in the farm's bucket (hack/sim-farm/config.toml's
s3_bucket, s3_region and aws_profile), each under
sim-data/<run>/<instance>/<seq>/ and, its bulky tables (simdb's
SAMPLE_TABLES), sim-samples/<run>/<instance>/<seq>/:

  pull(run, reads=False) -> Pull
                 `aws s3 sync` of the run's sim-data (and sim-samples with
                 `reads`) into the cache's farm/<run>/, its LISTs, GETs
                 and bytes a line of the farm's S3 ledger
  fetch_samples(plan, gets, got) -> set
                 the tables of simdb.sample_plan's `plan` ({src: (tables,
                 bytes, jobs)}), a GET a file, each checked against its
                 flush's manifest (size and sha256) and put in place
                 whole; returns the flushes S3 no longer holds, its GETs
                 and their bytes counted by run in `gets` and `got`
                 (Counters) for the caller's ledger line, however it ends
  farm_ledger(run, use)
                 appends a pull's line to the farm's S3 ledger
                 (target/sim-farm/s3.jsonl, farm.py's S3_LEDGER) under its
                 lock, as farm.py appends
  farm_bucket() -> (bucket, region, profile)
  aws(args, region, profile)
                 the aws CLI's `args` in the bucket's region, as its
                 profile

The cache and the main checkout are simdb's (simlab.simdb()).
"""

import contextlib
import dataclasses
import fcntl
import json
import math
import os
import subprocess
import sys
import time
import tomllib
from pathlib import Path

from . import engine, simdb


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


# --- the executor ---------------------------------------------------------------------


def execute(script, db=None, out="json", threads=None, memory=None):
    """Runs `script` as simdb.duck would in the CLI (the module docs)."""
    engine.need_module()
    sm = simdb()
    tmp = sm.cache_root() / "tmp"
    if tmp.parent.exists():
        tmp.mkdir(exist_ok=True)
    else:
        tmp = None
    try:
        conn = engine.store(db or ":memory:", threads, memory, tmp)
    except engine.duckdb.Error as e:
        raise sm.DuckError(str(e)) from None
    try:
        shown = engine.script(conn, script, text=out == "text")
    except (engine.QueryError, engine.duckdb.Error) as e:
        raise sm.DuckError(str(e)) from None
    finally:
        conn.close()
    if out == "json":
        return shown[-1].records() if shown else []
    if out == "text":
        return "".join(r.table() for r in shown)
    return None


def install():
    """Makes simdb.duck run its scripts in the module (`execute`)."""
    simdb().EXECUTOR = execute


@contextlib.contextmanager
def installed():
    """duck runs its scripts in the module inside the block, where the
    duckdb module imports (a script under the engine), else in the CLI;
    the executor before it is back after."""
    sm = simdb()
    before = sm.EXECUTOR
    if engine.duckdb is not None:
        sm.EXECUTOR = execute
    try:
        yield
    finally:
        sm.EXECUTOR = before


# --- S3 -------------------------------------------------------------------------------

# The aws CLI's multipart size: it downloads a file past it a ranged GET a
# part (farm.py's CLI_PART).
CLI_PART = 8 * 1024 * 1024


def farm_bucket():
    cfg = simdb().main_checkout() / "hack" / "sim-farm" / "config.toml"
    if not cfg.exists():
        sys.exit(f"no {cfg}: the farm's bucket is its s3_bucket")
    c = tomllib.loads(cfg.read_text())
    return c["s3_bucket"], c.get("s3_region"), c.get("aws_profile")


def aws(args, region, profile):
    """Runs the aws CLI's `args` in the farm bucket's region, as its
    profile."""
    return subprocess.run(["aws", *args, *(["--region", region] if region else []),
                           *(["--profile", profile] if profile else [])], capture_output=True, text=True)


def pull(run, reads=False):
    """Copies farm run `run`'s flushes from S3 into the cache
    (farm/<run>/<instance>/<seq>/), and with `reads` their samples too.
    What it cost S3 goes to the run's line of the farm's S3 ledger: a LIST
    a thousand of a prefix's keys, a GET a file it downloaded (a part each
    past CLI_PART), and the bytes, out to the internet."""
    sm = simdb()
    bucket, region, profile = farm_bucket()
    cache = sm.cache_root() / "cache"
    dest = cache / "farm" / run
    lists = gets = got = 0
    pulled = set()
    for prefix in ["sim-data"] + (["sim-samples"] if reads else []):
        p = aws(["s3", "sync", f"s3://{bucket}/{prefix}/{run}/", str(dest), "--no-progress"], region, profile)
        p.check_returncode()
        for line in p.stdout.splitlines():
            if line.startswith("download: ") and " to " in line:
                to = Path(line.rsplit(" to ", 1)[1])
                size = to.stat().st_size
                gets += 1 if size < CLI_PART else math.ceil(size / CLI_PART)
                got += size
                pulled.add(to.parent.relative_to(cache).as_posix())
        keys = sum(1 for f in dest.glob("**/*")
                   if f.is_file() and (f.stem in sm.SAMPLE_TABLES) == (prefix == "sim-samples"))
        lists += max(1, math.ceil(keys / 1000))
    use = {"region": region or "", "tier1": {"pull_list": lists}, "tier2": {"pull_get": gets}, "up": {},
           "down": {"internet": got}, "stored": 0}
    farm_ledger(run, use)
    return Pull(run, dest, sorted(pulled), use)


def farm_ledger(run, use):
    """A pull's line of the farm's S3 ledger, appended under its lock as
    farm.py appends (append_jsonl)."""
    path = simdb().main_checkout() / "target" / "sim-farm" / "s3.jsonl"
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


def fetch_samples(plan, gets, got):
    """Fetches `plan`'s tables (simdb.sample_plan's) from S3 into the
    cache, a GET a file (s3api get-object: one request whatever the
    size), each checked against its flush's manifest and put in place
    whole; returns the flushes S3 no longer holds. Counts each run's GETs
    (a missing key's too) in `gets` and their bytes in `got`, however it
    ends."""
    sm = simdb()
    bucket, region, profile = farm_bucket()
    root = sm.cache_root() / "cache"
    gone = set()
    for src, (tables, _, _) in plan.items():
        _, run, flush = src.split("/", 2)
        want = sm.manifest_of(src)["tables"]
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
                if part.stat().st_size != want[t]["bytes"] or sm.sha256(part) != want[t]["sha256"]:
                    sys.exit(f"{src}: S3's {t}.parquet ({part.stat().st_size} bytes) isn't its manifest's")
                part.replace(dest)
            finally:
                part.unlink(missing_ok=True)
    return gone
