# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Testbed runs in the lake: hack/testbed/run.sh's out dirs, each read as a
run of its twin's cell and folded into the cache as a local suite
invocation's jobs are, so the catalog's tables, verdicts, scoring macros
and views read a testbed run as they read a sim run.

strims-sim's testbed module does the reading (its testbed_tables example):
it writes a run's tables with the sim's own table writer, from the
records the testbed's nodes log and the run's captures, and its
`job.json` names the run's key, its sim (`testbed`), its test
(`calibration_star`), its twin's knobs (its env) and its row. A run's
cell is the tier cell (suite.TIERS) whose env its knobs are, but for the
knobs a testbed run always sets and a tier's cells don't (TESTBED_FIXED:
one clock, its tbf's buckets, honest viewers) and the job keys; a run of no
tier cell takes a cell named for its knobs (`tb-<hash>`).

  tables(run_dir, binary=None, knobs=None) -> Path
                 the run's tables dir (<run_dir>/tables), written by the
                 testbed_tables binary (BINARY's, by default this
                 checkout's release build), `knobs` added to its twin's
  cell_of(env) -> str | None
                 the tier cell whose env `env` is (see above), or None
  ingest(run_dirs, name, binary=None, cell=None, arm="testbed", knobs=None)
        -> (src, [job])
                 the runs' tables folded into the cache as source
                 testbed/<name>; each run a job of `cell` (else cell_of's,
                 else its knobs' name) and `arm`, its seed its place among
                 the runs of its cell (1, 2, ...), as a twin's seeds pair
                 with it
  twins(src, binary, name, seeds=8, arm="twin", env=None) -> (src, [job])
                 each cell of testbed source `src`'s twin run at seeds
                 1..N (a scenarios test binary's calibration_star with the
                 cell's knobs and `env`'s), folded into the cache as
                 source testbed/<name>: the scoring macros then pair a
                 twin seed with a testbed run of its number (effects,
                 pairs: testbed/<name> arm `arm` against src arm testbed)
  cells_in(src) -> {cell: env}
                 a testbed source's cells and their twins' knobs

hack/sim-db/simdb.py testbed and twin are its command line; `simdb.py index` then
takes the source in. A testbed source is a local experiment (exp_id its
source), its arm `arm`.
"""

import hashlib
import json
import shutil
import subprocess
import tempfile
from pathlib import Path

from . import store_host

ROOT = Path(__file__).resolve().parents[2]
BINARY = ROOT / "target" / "release" / "examples" / "testbed_tables"
# What a testbed run's knobs hold that its tier cell's env doesn't, or
# holds otherwise: one clock (a cell draws real skew), its tbf's buckets
# (a cell has none), honest viewers (a tier's mix or off), the qdisc tree
# and the settings no twin knob takes (TESTBED_*); and the job keys.
TESTBED_FIXED = ("STRIMS_SIM_CLOCK_SKEW", "STRIMS_SIM_CAL_SEED_BURST", "STRIMS_SIM_CAL_VIEWER_BURST",
                 "STRIMS_SIM_HOSTILE")
JOB_KNOBS = ("STRIMS_SIM_JOB", "STRIMS_SIM_SERIES", "STRIMS_SIM_SEED", "STRIMS_SIM_OUT")


def tables(run_dir, binary=None, knobs=None):
    """The run's tables, written into <run_dir>/tables by testbed_tables."""
    run_dir = Path(run_dir)
    out = run_dir / "tables"
    shutil.rmtree(out, ignore_errors=True)
    binary = Path(binary or BINARY)
    if not binary.exists():
        raise FileNotFoundError(f"no testbed_tables at {binary}: cargo build --release -p strims-sim "
                                f"--example testbed_tables")
    args = [str(binary), str(run_dir), str(out)]
    for k, v in sorted((knobs or {}).items()):
        args += ["--knob", f"{k}={v}"]
    p = subprocess.run(args, capture_output=True, text=True)
    if p.returncode:
        raise RuntimeError(f"{run_dir}: {p.stderr.strip()}")
    return out


def comparable(env):
    """`env` less TESTBED_FIXED, the TESTBED_ settings and the job keys."""
    return {k: v for k, v in env.items()
            if k not in TESTBED_FIXED and k not in JOB_KNOBS and not k.startswith("TESTBED_")}


def cell_of(env):
    """The name of the calibration_star tier cell whose env is `env` (see
    the module docs), or None."""
    from . import suite

    if any(k.startswith("TESTBED_") and k != "TESTBED_QDISC" for k in env):
        return None
    want = comparable(env)
    for cells in suite.TIERS.values():
        for c in cells:
            if c["test"] == "calibration_star" and comparable(c["env"]) == want:
                return c["name"]
    return None


def knob_cell(env):
    """A cell's name for a run of no tier cell: its knobs' hash."""
    text = "\n".join(f"{k}={v}" for k, v in sorted(env.items()))
    return "tb-" + hashlib.sha256(text.encode()).hexdigest()[:8]


def ingest(run_dirs, name, binary=None, cell=None, arm="testbed", knobs=None):
    """The runs folded into the cache as source testbed/<name> (see the
    module docs); returns the source and the jobs."""
    src = f"testbed/{name}"
    jobs, seeds = [], {}
    for d in run_dirs:
        out = tables(d, binary, knobs)
        job = json.loads((out / "job.json").read_text())
        c = cell or cell_of(job["env"]) or knob_cell(job["env"])
        seeds[c] = seeds.get(c, 0) + 1
        jobs.append({**job, "dir": str(out), "cell": c, "arm": arm, "seed": seeds[c],
                     "at": None, "wall_s": None, "rss_mb": None})
    export(src, jobs)
    return src, jobs


def export(src, jobs):
    """`jobs` folded into a store and exported to the cache as `src`."""
    (store_host.cache_root() / "tmp").mkdir(parents=True, exist_ok=True)
    work = Path(tempfile.mkdtemp(prefix="testbed-", dir=store_host.cache_root() / "tmp"))
    try:
        db = work / "store.duckdb"
        store_host.fold(db, jobs, src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
    finally:
        shutil.rmtree(work, ignore_errors=True)


def cells_in(src):
    """The cells of a testbed source's runs, {cell: env}: their twins'
    knobs."""
    runs = store_host.cache_root() / "cache" / src / "runs.parquet"
    rows = store_host.duck(f"SELECT DISTINCT cell, env FROM read_parquet({store_host.q(runs)}) "
                           "WHERE sim = 'testbed' ORDER BY cell;")
    return {r["cell"]: json.loads(r["env"]) for r in rows}


def twins(src, binary, name, seeds=8, arm="twin", env=None, parallel=8):
    """Each cell of testbed source `src`'s twin: strims-sim's
    calibration_star (the scenarios test binary `binary`) at seeds 1..N
    with the cell's knobs (its TESTBED_ settings left out) and `env`'s,
    one farm job's way each (STRIMS_SIM_JOB=1), their tables folded into
    the cache as source testbed/<name>, each a job of its cell, `arm` and
    seed. Returns the source and the jobs."""
    import concurrent.futures

    binary = Path(binary)
    with open(binary, "rb") as f:
        digest = hashlib.file_digest(f, "sha256").hexdigest()
    work = Path(tempfile.mkdtemp(prefix="twins-", dir=ROOT / "target"))
    todo = []
    for cell, knobs in cells_in(src).items():
        base = {k: v for k, v in knobs.items() if not k.startswith("TESTBED_")}
        for seed in range(1, seeds + 1):
            todo.append((cell, {**base, **(env or {}), "STRIMS_SIM_JOB": "1"}, seed))

    def one(t):
        cell, e, seed = t
        key = json.dumps([digest, sorted(e.items()), seed])
        k = "tw-" + hashlib.sha256(key.encode()).hexdigest()[:16]
        out = work / k
        run = {**e, "STRIMS_SIM_SEED": str(seed), "STRIMS_SIM_OUT": str(out)}
        p = subprocess.run([str(binary), "--exact", "calibration_star", "--include-ignored", "--nocapture",
                            "--test-threads", "1"], env={**_base_env(), **run}, capture_output=True, text=True)
        rows = [line[line.find("ROW {") + 4:] for line in p.stdout.splitlines() if "ROW {" in line]
        if p.returncode or len(rows) != 1:
            raise RuntimeError(f"{cell} seed {seed}: exit {p.returncode}\n{p.stderr[-2000:]}")
        return {"k": k, "dir": str(out), "sim": "strims-sim", "test": "calibration_star", "cell": cell, "arm": arm,
                "seed": seed, "env": e, "row": json.loads(rows[0]), "binary_sha256": digest}

    try:
        with concurrent.futures.ThreadPoolExecutor(parallel) as ex:
            jobs = list(ex.map(one, todo))
        out = f"testbed/{name}"
        export(out, jobs)
    finally:
        shutil.rmtree(work, ignore_errors=True)
    return out, jobs


def _base_env():
    """The environment a twin runs in: PATH and HOME alone, as a farm job's."""
    import os

    return {k: os.environ[k] for k in ("PATH", "HOME") if k in os.environ}
