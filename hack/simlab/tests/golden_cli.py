#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""What hack/sim-db/simdb.py and hack/capture/analyze.py's verdict
commands print over a fixture cache, invocation by invocation, against
the text testdata/golden/cli.txt.gz holds:

  python3 hack/simlab/tests/golden_cli.py check [--keep DIR]
  python3 hack/simlab/tests/golden_cli.py capture

The fixture is a cache root and a main checkout of its own, made fresh in
a scratch dir each run: jobs of both sims (a viewer's lag and reads each
second laid out by simdb_test's audience(), a collapse, a lag rise, a
seed's uplink step, trickle arrivals, a departure; events, stalls, bins,
parents, node_bins and a forecast log beside them), written as the sims
write their output dirs and folded and exported by simdb.py itself, under
two local suite invocations and a suite `--sets` farm run of two matrices
(its flushes, one lacking its reads as one pulled without --reads does;
its matrices' indexes, one with a row the farm reused from another run;
its farm-runs index), with stand-ins for farm.py's prices and config. Each
invocation runs as a live caller runs it, `python3 SCRIPT ARGS`, with
SIM_DB_ROOT the fixture's cache root and GIT_DIR an empty repository at the
fixture's root (main_checkout's), so the farm's and the suite's state come
from the fixture and nothing reaches S3 or the main checkout; TZ is UTC
and COLUMNS 100. Each output has the fixture's root as <ROOT> and the
home directory as <HOME>; the volatile fields (an export's time and its
files' bytes and hashes; the catalog's times of its own making) read
<VOLATILE>. `index --quiet` is followed by every catalog table, as JSON.

`check` exits 1 on any difference, printing a diff of each invocation
that differs; `capture` rewrites it from today's code.
"""

import argparse
import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import goldens

HACK = Path(__file__).resolve().parents[2]
SIMDB = HACK / "sim-db" / "simdb.py"
ANALYZE = HACK / "capture" / "analyze.py"
GOLDEN = goldens.TESTDATA / "cli.txt.gz"
SECS = 40
AT = "2026-11-01T12:00:00Z"
BIN_MS = 100


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_simdb_test = None


def simdb_test():
    """hack/sim-db/simdb_test.py, whose fixtures (audience, the Queries'
    forecast log) the jobs are made of, and its simdb."""
    global _simdb_test
    if _simdb_test is None:
        _simdb_test = load(HACK / "sim-db" / "simdb_test.py", "simdb_test")
    return _simdb_test


# --- the jobs -------------------------------------------------------------------------

T0 = [(None, None)] * 5
TRICKLE = [(None, None), (None, None), (3, None), (6, None), (9, 31)]
PP_ENV = {"SIM_STREAMS": "16", "SIM_CHUNK": "4096", "SIM_SERIES": "1"}
SS_ENV = {"STRIMS_SIM_JOB": "1", "STRIMS_SIM_TCP": "1", "STRIMS_SIM_SERIES": "1"}
# (src, k, sim, test, cell, arm, seed, env, stays, shape): shape is how the
# run goes wrong, if it does.
FARM = "farm/sf0001-00+1more-aaaa"
OTHER = "farm/se0001-01+0more-cccc"
JOBS = [
    ("suite/wt/inv1", "pp-base:star-5:1", "ppspp", "sim_job", "star-5", "ppspp-base", 1, PP_ENV, T0, None),
    ("suite/wt/inv1", "pp-tree:star-5:1", "ppspp", "sim_job", "star-5", "ppspp-tree", 1, PP_ENV, T0, "collapse"),
    ("suite/wt/inv1", "ss-base:halve-5:1", "strims-sim", "calibration_star", "halve-5", "ss-base", 1, SS_ENV,
     TRICKLE, "step"),
    ("suite/wt/inv1", "ss-tree:halve-5:1", "strims-sim", "calibration_star", "halve-5", "ss-tree", 1, SS_ENV,
     TRICKLE, "rise"),
    ("suite/wt/inv2", "pp-base:star-5:2", "ppspp", "sim_job", "star-5", "ppspp-base", 2, PP_ENV, TRICKLE, None),
    ("suite/wt/inv2", "pp-tree:star-5:2", "ppspp", "sim_job", "star-5", "ppspp-tree", 2, PP_ENV, TRICKLE, "rise"),
    # the second suite invocation holds a copy of the first's job
    ("suite/wt/inv2", "pp-base:star-5:1", "ppspp", "sim_job", "star-5", "ppspp-base", 1, PP_ENV, T0, None),
    (f"{FARM}/i-1/0", "f-off-2001", "strims-ppspp:sim", "sim_job", "sf0001-00", "x", 2001,
     {**PP_ENV, "SIM_EXP": "off", "SIM_SEED": "2001"}, T0, None),
    (f"{FARM}/i-1/0", "f-a-2001", "strims-ppspp:sim", "sim_job", "sf0001-00", "x", 2001,
     {**PP_ENV, "SIM_EXP": "a", "SIM_SEED": "2001"}, T0, "collapse"),
    (f"{FARM}/i-1/0", "g-off-2001", "strims-sim:scenarios", "calibration_star", "sf0001-01", "x", 2001,
     {**SS_ENV, "STRIMS_SIM_EXP": "off", "STRIMS_SIM_SEED": "2001"}, TRICKLE, "step"),
    (f"{FARM}/i-1/0", "g-a-2001", "strims-sim:scenarios", "calibration_star", "sf0001-01", "x", 2001,
     {**SS_ENV, "STRIMS_SIM_EXP": "a", "STRIMS_SIM_SEED": "2001"}, TRICKLE, None),
    # a second instance's flush, pulled without its reads
    (f"{FARM}/i-2/3", "f-off-2002", "strims-ppspp:sim", "sim_job", "sf0001-00", "x", 2002,
     {**PP_ENV, "SIM_EXP": "off", "SIM_SEED": "2002"}, TRICKLE, None),
    (f"{FARM}/i-2/3", "f-a-2002", "strims-ppspp:sim", "sim_job", "sf0001-00", "x", 2002,
     {**PP_ENV, "SIM_EXP": "a", "SIM_SEED": "2002"}, TRICKLE, "rise"),
    # a job of another run, which sf0001-01 reused
    (f"{OTHER}/i-1/0", "g-off-2002", "strims-sim:scenarios", "calibration_star", "se0001-01", "x", 2002,
     {**SS_ENV, "STRIMS_SIM_EXP": "off", "STRIMS_SIM_SEED": "2002"}, T0, None),
]
# The flushes whose reads S3 keeps but the cache lacks.
NO_READS = {f"{FARM}/i-2/3"}
# The jobs with a forecast log and per-node bins.
FORECAST = {"pp-base:star-5:1", "f-off-2001"}


def series(sim, stays, shape):
    """A job's series: audience()'s, then the shape's trouble: every
    viewer's lag at 6 s from 18 s to 30 s (a collapse), rising 300 ms a
    second from 15 s (a lag rise), or the seed's uplink cut at 20 s and back
    at 32 s (a step)."""
    s = simdb_test().audience(SECS, stays, sim=sim)
    s["seed_up"] = [[0, 8_000_000]]
    for lag in s["lag"]:
        for k in range(SECS):
            if lag[k] < 0:
                continue
            if shape == "collapse" and 18 <= k < 30:
                lag[k] = 6000
            elif shape == "rise" and k >= 15:
                lag[k] += 300 * (k - 14)
    if shape == "step":
        s["seed_up"] = [[0, 8_000_000], [20, 2_000_000], [32, 8_000_000]]
    return s


def write_table(d, name, cols, rows):
    (d / f"{name}.schema.json").write_text(json.dumps([[n, t] for n, t in cols]))
    with open(d / f"{name}.csv", "w") as f:
        f.write(",".join(n for n, _ in cols) + "\n")
        for r in rows:
            f.write(",".join("" if x is None else str(x).lower() if isinstance(x, bool) else str(x) for x in r) + "\n")


def job_dir(d, job):
    """Writes `job`'s output dir as its sim writes it, and returns the job
    simdb.fold takes."""
    src, k, sim, test, cell, arm, seed, env, stays, shape = job
    ss = sim.startswith("strims-sim")
    s = series("strims-sim" if ss else "ppspp", stays, shape)
    d.mkdir(parents=True)
    n = len(stays)
    up = lambda t: [u for at, u in s["seed_up"] if at <= t][-1]
    seconds = [(t, s["present"][t], s["hits"][t], 10 * s["present"][t], int(t % 7 == 3), s["present"][t],
                up(t) * 9 // 10, up(t), 600 + 10 * (t % 5), min(3, s["present"][t]), 8 * s["present"][t])
               for t in range(SECS)]
    write_table(d, "seconds", [("sec", "BIGINT"), ("present", "BIGINT"), ("hits", "BIGINT"), ("sent", "BIGINT"),
                               ("pruned", "BIGINT"), ("parents", "BIGINT"), ("capacity", "BIGINT"),
                               ("avail", "BIGINT"), ("busy", "BIGINT"), ("copies", "BIGINT"), ("pushed", "BIGINT")],
                seconds)
    vs = [(v, t, s["lag"][v][t], s["frames"][v][t]) for v in range(n) for t in range(SECS)
          if not (ss and s["frames"][v][t] < 0)]
    write_table(d, "viewer_seconds", [("viewer", "BIGINT"), ("sec", "BIGINT"), ("lag_ms", "BIGINT"),
                                      ("frames", "BIGINT")], vs)
    start = -7 if ss else -1000
    arrive = [start if a < 0 else a for a in s["arrive_ms"]]
    if ss:
        write_table(d, "viewers", [("viewer", "BIGINT"), ("host", "BIGINT"), ("name", "VARCHAR"), ("up", "BIGINT"),
                                   ("hostile", "BOOLEAN"), ("churned", "BOOLEAN"), ("arrive_ms", "BIGINT"),
                                   ("depart_ms", "BIGINT")],
                    [(v, v + 1, f"v{v}", 2_000_000, False, s["depart_ms"][v] is not None, arrive[v],
                      s["depart_ms"][v]) for v in range(n)])
    else:
        write_table(d, "viewers", [("viewer", "BIGINT"), ("node", "BIGINT"), ("up", "BIGINT"), ("hostile", "BOOLEAN"),
                                   ("churned", "BOOLEAN"), ("arrive_ms", "BIGINT"), ("depart_ms", "BIGINT")],
                    [(v, v + 1, 2_000_000, False, s["depart_ms"][v] is not None, arrive[v], s["depart_ms"][v])
                     for v in range(n)])
    # A read every 250 ms of each second a viewer reads, at its lag.
    reads = [(v, t * 1000 + q * 250, 4 * t + q, s["lag"][v][t]) for v in range(n) for t in range(SECS)
             for q in range(4) if s["lag"][v][t] >= 0 and s["frames"][v][t] > 0]
    write_table(d, "reads", [("viewer", "BIGINT"), ("t_ms", "BIGINT"), ("seq", "BIGINT"), ("lag_ms", "BIGINT")],
                reads)
    events = []
    for v, (a, dep) in enumerate(stays):
        node = v + 1
        events.append((0 if a is None else a * 1_000_000, "join", node, 0, 0, ""))
        first = next((t for t in range(SECS) if s["frames"][v][t] > 0), None)
        if first is not None:
            events.append((first * 1_000_000 + 250_000, "first_read", node, -1, 0, ""))
            events.append((first * 1_000_000 + 900_000, "caught_up", node, -1, 0, ""))
        if dep is not None:
            events.append((dep * 1_000_000, "depart", node, -1, 0, "graceful"))
    if shape == "collapse":
        events += [(18_000_000, "parent_departed", 2, 1, 1, "abrupt from=1"),
                   (24_000_000, "parent_departed", 3, 1, 2, "abrupt from=1")]
    if shape == "step":
        # The seed's uplink as a level: its rate from the run's start, then each step.
        events += [(start * 1000, "uplink_rate", 0, -1, 8_000_000, ""),
                   (20_000_000, "uplink_rate", 0, -1, 2_000_000, ""),
                   (32_000_000, "uplink_rate", 0, -1, 8_000_000, "")]
    events.append((5_000_000, "latency", 0, -1, 250, "to=3 wait_ms=12"))
    write_table(d, "events", [("t_us", "BIGINT"), ("kind", "VARCHAR"), ("node", "BIGINT"), ("peer", "BIGINT"),
                              ("value", "BIGINT"), ("detail", "VARCHAR")], sorted(events))
    stalls = []
    if shape in ("collapse", "rise"):
        stalls = [(v, delay, 18_000 if shape == "collapse" else 22_000, 4750 if delay == 1250 else 1750)
                  for v in range(n) for delay in (1250, 4250) if not (shape == "rise" and delay == 4250)]
    write_table(d, "stalls", [("viewer", "BIGINT"), ("delay_ms", "BIGINT"), ("t_ms", "BIGINT"), ("stall_ms", "BIGINT")],
                stalls)
    bins = [(b, b * BIN_MS, up(b * BIN_MS // 1000) * 9 // 10 if b % 5 == 0 else -1, up(b * BIN_MS // 1000) // 80,
             int(shape == "collapse" and 180 <= b < 300)) for b in range(SECS * 1000 // BIN_MS)]
    write_table(d, "bins", [("bin", "BIGINT"), ("t_ms", "BIGINT"), ("seed_meter", "BIGINT"),
                            ("seed_up_bytes", "BIGINT"), ("viewer_timeouts", "BIGINT")], bins)
    parents = [(0 if a is None else a * 1000, v + 1, st, 0, "open") for v, (a, _) in enumerate(stays) for st in (0, 1)]
    if shape == "collapse":
        parents += [(18_000, 2, 0, -1, "lost"), (18_400, 2, 0, 3, "switch"), (24_000, 3, 1, 4, "shed")]
    write_table(d, "parents", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("stream", "BIGINT"), ("parent", "BIGINT"),
                               ("reason", "VARCHAR")], sorted(parents))
    if k in FORECAST:
        # The Queries' log, with the estimate's horizon and samples, the
        # stream, the chunk, its pace and the channel's opening beside.
        f = [(*r, i % 3, 40 + i, r[8] + 30, 3 + i, i % 2 == 0, i % 3 == 0, 1000)
             for i, r in enumerate(simdb_test().FORECASTS)]
        write_table(d, "forecast", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"), ("peer", "BIGINT"),
                                    ("peer_honest", "BOOLEAN"), ("live", "BOOLEAN"), ("run", "BIGINT"),
                                    ("first_sight", "BOOLEAN"), ("mean_ms", "BIGINT"), ("held_ms", "BIGINT"),
                                    ("seen_ms", "BIGINT"), ("sample_ms", "BIGINT"), ("stream", "BIGINT"),
                                    ("seq", "BIGINT"), ("horizon_ms", "BIGINT"), ("samples", "BIGINT"),
                                    ("fresh", "BOOLEAN"), ("lazy", "BOOLEAN"), ("channel_open_ms", "BIGINT")], f)
        write_table(d, "node_bins", [("bin", "BIGINT"), ("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"),
                                     ("hits", "BIGINT")],
                    [(t // BIN_MS, t, node, v, int(v >= 0 and t >= start_ms))
                     for node, v, start_ms in ((1, 0, 1000), (2, 1, 3000), (0, -1, 0))
                     for t in range(0, 5000, BIN_MS)])
    meta = {"sim": sim, "seconds": SECS, "viewers": n, "bin_ms": BIN_MS,
            **{x: s[x] for x in ("rate", "chunk", "frame", "secs", "seed_up", "viewer_up")}}
    (d / "series_meta.json").write_text(json.dumps(meta))
    lags = sorted(x for lag in s["lag"] for x in lag if x >= 0)
    q = lambda p: lags[min(len(lags) - 1, int(p * len(lags)))]
    n_reads = len(reads)
    row = {"passed": shape is None, "frames": n_reads, "bytes": n_reads * 4096, "trace": f"tr-{k}",
           "reads_hash": f"rh-{len(k)}-{n_reads}", "lag_p50_ms": q(0.5), "lag_p99_ms": q(0.99),
           "lag_p99_ms_honest": q(0.99), "join_p90_ms": 900 + 10 * seed, "caught_up_p90_ms": 1400 + 20 * seed,
           "useful_bytes": n_reads * 4096, "wire_bytes": n_reads * 4400 + 1000 * seed,
           "meter_seeds": [1.5, 2.0],
           "arrivals": "trickle" if stays is TRICKLE else "t0", "series": {"secs": SECS},
           **({"off_drop": [2.5, 3.0], "off_rise": [4.0]} if shape == "step" else {})}
    return {"k": k, "dir": str(d), "sim": sim, "test": test, "cell": cell, "arm": arm, "seed": seed, "env": env,
            "row": row, "wall_s": 1.5 + seed / 1000, "rss_mb": 40.0 + len(k), "at": AT}


def build(root):
    """Makes the fixture under `root`: its cache (target/sim-db), the
    farm's and the suite's state, stand-ins for farm.py and its config, and
    an empty git repository (the fixture's main checkout)."""
    sm = simdb_test().simdb
    os.environ["SIM_DB_ROOT"] = str(root / "target" / "sim-db")
    cache = root / "target" / "sim-db" / "cache"
    jobs_dir = root / "jobs"
    by_src = {}
    for i, job in enumerate(JOBS):
        by_src.setdefault(job[0], []).append(job_dir(jobs_dir / f"{i:02d}", job))
    (root / "stores").mkdir()
    for n, (src, jobs) in enumerate(sorted(by_src.items())):
        db = root / "stores" / f"s{n}.duckdb"
        sm.fold(db, jobs, src)
        dest = cache / src
        sm.export(db, dest)
        m = json.loads((dest / "manifest.json").read_text())
        m["at"] = f"2026-11-0{n + 1}T12:00:00Z"
        if src in NO_READS:
            (dest / "reads.parquet").unlink()
        (dest / "manifest.json").write_text(json.dumps(m, indent=1) + "\n")
    # The suite's farm run: two matrices, a --sets run (knob and values).
    runs = root / "target" / "sim-suite" / "farm-runs"
    runs.mkdir(parents=True)
    (runs / "sf0001.json").write_text(json.dumps({
        "cells": [{"name": "star-5", "sim": "ppspp", "test": "sim_job", "why": "a star", "env": PP_ENV,
                   "mem_gib": 2},
                  {"name": "halve-5", "sim": "strims-sim", "test": "calibration_star", "why": "a halving",
                   "env": SS_ENV, "mem_gib": 2}],
        "knob": {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"}, "values": ["off", "a"],
        "arms": {"x": "1" * 40}, "seeds": [2001, 2002], "base": "off", "question": "does a help?"}) + "\n")
    farm = root / "target" / "sim-farm"
    (farm / FARM.split("/")[1]).mkdir(parents=True)
    (farm / FARM.split("/")[1] / "matrices").write_text("sf0001-00\nsf0001-01\n")
    lines = {}
    for i, (src, k, sim, test, cell, arm, seed, env, *_rest) in enumerate(JOBS):
        if not src.startswith(FARM):
            continue
        lines.setdefault(cell, []).append({
            "id": f"j{i}", "key": k, "arm": arm, "commit": "1" * 40, "binary": "b" * 64, "test": test, "env": env,
            "src": src, "at": AT, "wall_s": 1.5, "max_rss_mb": 40, "row": {"trace": f"tr-{k}"},
            "reads": True})
    lines["sf0001-01"].append({"id": "j99", "key": "g-off-2002", "arm": "x", "commit": "1" * 40, "binary": "b" * 64,
                               "test": "calibration_star", "env": JOBS[-1][7], "src": f"{OTHER}/i-1/0",
                               "from": "se0001-01", "at": AT, "wall_s": 1.5, "max_rss_mb": 40, "row": {},
                               "reads": True})
    for m, recs in lines.items():
        (farm / m).mkdir(parents=True, exist_ok=True)
        (farm / m / "index.jsonl").write_text("".join(json.dumps(r) + "\n" for r in recs))
    (farm / "costs.jsonl").write_text(json.dumps({"run": FARM.split("/")[1], "kind": "test", "at": AT, "cost": 0.25,
                                                  "jobs": 7, "wall_s": 300}) + "\n")
    (farm / "s3.jsonl").write_text(json.dumps({"run": FARM.split("/")[1], "at": AT, "source": "pull", "region": "r",
                                               "tier1": {"pull_list": 2}, "tier2": {"pull_get": 10}, "up": {},
                                               "down": {"internet": 1 << 20}, "stored": 0}) + "\n")
    hf = root / "hack" / "sim-farm"
    hf.mkdir(parents=True)
    (hf / "farm.py").write_text("S3_TIER1 = 0.0055 / 1000\nS3_TIER2 = 0.0044 / 10000\nS3_STORAGE_GB_MONTH = 0.026\n"
                                "S3_KEPT_DAYS = 7\nS3_INTER_REGION_GB = 0.02\nS3_INTERNET_GB = 0.09\nGB = 1 << 30\n")
    (hf / "config.toml").write_text('s3_bucket = "golden-bucket"\ns3_region = "us-west-1"\n')
    # The aws CLI the invocations find: S3 is never reached from here.
    (root / "bin").mkdir()
    (root / "bin" / "aws").write_text("#!/bin/sh\necho \"aws stand-in: no S3 in the golden ($1 $2)\" >&2\nexit 1\n")
    (root / "bin" / "aws").chmod(0o755)
    subprocess.run(["git", "init", "-q", str(root)], check=True)
    shutil.rmtree(root / "stores")
    return jobs_dir


# --- the invocations ----------------------------------------------------------------

S = ["--src", "suite/**"]
FARM_MATRICES = ["<ROOT>/target/sim-farm/sf0001-00", "<ROOT>/target/sim-farm/sf0001-01"]
# (name, script, args): <ROOT> is the fixture's root, <JOBS> its jobs' dirs.
INVOCATIONS = [
    ("help", SIMDB, ["--help"]),
    ("sql-help", SIMDB, ["sql", "--help"]),
    ("duckdb", SIMDB, ["duckdb"]),
    ("sql-runs", SIMDB, ["sql", "SELECT src, k, cell, arm, seed FROM runs ORDER BY ALL"]),
    ("sql-runs-json", SIMDB, ["sql", "SELECT src, k, cell, arm, seed, bin_ms FROM runs ORDER BY ALL", "--json"]),
    ("sql-cell-glob", SIMDB, ["sql", "SELECT k, cell, arm FROM runs ORDER BY ALL", "--cell", "star-5*",
                              "--arm", "a", "--arm", "ppspp-*"]),
    ("sql-seed", SIMDB, ["sql", "SELECT k, cell, arm, seed FROM runs ORDER BY ALL;", "--seed", "2001", "--seed", "1"]),
    ("sql-reads", SIMDB, ["sql", "SELECT r.cell, r.arm, count(*) AS n, avg(lag_ms) AS lag, max(t_ms) AS last "
                                 "FROM reads JOIN runs r USING (k, src) GROUP BY ALL ORDER BY ALL", *S]),
    ("sql-types", SIMDB, ["sql", "SELECT 1::HUGEINT AS h, 1.25::DECIMAL(5,2) AS d, 'nan'::DOUBLE AS n, "
                                 "[1, 2] AS l, {'a': 1} AS s, MAP {'k': 2} AS m, TIMESTAMPTZ '2026-11-10 10:00:00+00' "
                                 "AS t, NULL AS z", "--json"]),
    ("sql-types-text", SIMDB, ["sql", "SELECT 1::HUGEINT AS h, 1.25::DECIMAL(5,2) AS d, 'nan'::DOUBLE AS n, "
                                      "[1, 2] AS l, {'a': 1} AS s, MAP {'k': 2} AS m, NULL AS z, 'x' AS z"]),
    ("sql-verdicts", SIMDB, ["sql", "SELECT * EXCLUDE (src) FROM verdicts ORDER BY k", "--json"]),
    ("sql-episodes", SIMDB, ["sql", "SELECT k, kind, start, stop, len FROM episodes ORDER BY ALL"]),
    ("sql-phases", SIMDB, ["sql", "SELECT k, phase, count(*) AS secs, sum(healthy::INT) AS healthy, "
                                  "round(avg(lag_q), 1) AS lag_q FROM verdict_seconds GROUP BY ALL ORDER BY ALL"]),
    ("sql-derived-events", SIMDB, ["sql", "SELECT k, sec, kind, value FROM derived_events ORDER BY ALL", "--json"]),
    ("sql-playback", SIMDB, ["sql", "SELECT * EXCLUDE (src) FROM playback(1250) ORDER BY ALL LIMIT 20", *S]),
    ("sql-stalls-parents", SIMDB, ["sql", "SELECT 'stalls' AS t, count(*) AS n FROM stalls UNION ALL "
                                          "SELECT 'parents', count(*) FROM parents UNION ALL "
                                          "SELECT 'node_bins', count(*) FROM node_bins UNION ALL "
                                          "SELECT 'bins', count(*) FROM bins ORDER BY ALL"]),
    ("sql-viewers", SIMDB, ["sql", "SELECT k, viewer, node, churned, arrive_ms, depart_ms FROM viewers "
                                   "ORDER BY ALL LIMIT 12", "--json"]),
    ("sql-scalars", SIMDB, ["sql", "SELECT name, count(*) AS n, sum(value) AS total FROM scalars "
                                   "GROUP BY ALL ORDER BY ALL"]),
    ("sql-since", SIMDB, ["sql", "SELECT k, count(*) AS n FROM reads GROUP BY ALL ORDER BY ALL", "--src",
                          "farm/sf0001*/i-1/*", "--since", "10", "--until", "20"]),
    ("sql-error", SIMDB, ["sql", "SELECT nope FROM runs"]),
    ("pooled", SIMDB, ["pooled", *S]),
    ("pooled-phase", SIMDB, ["pooled", "--by", "phase", *S]),
    ("pooled-bin", SIMDB, ["pooled", "--by", "bin", "--bin", "10000", *S]),
    ("pooled-window", SIMDB, ["pooled", "--since", "10", "--until", "30", "--cell", "halve-5*", "--src",
                              "suite/**", "--src", "farm/sf0001*/i-1/*"]),
    ("aligned-onset", SIMDB, ["aligned", "onset", *S]),
    ("aligned-seed-down", SIMDB, ["aligned", "seed_down", "--field", "lag_q", "--before", "5", "--after", "20"]),
    ("aligned-collapse", SIMDB, ["aligned", "collapse", "--field", "healthy", "--bin", "2000", "--json"]),
    ("aligned-present-join", SIMDB, ["aligned", "present_join", "--nth", "2", "--field", "hit_share",
                                     "--after", "10"]),
    ("aligned-join-node", SIMDB, ["aligned", "join", "--per", "node", "--field", "lag_p99", "--after", "8", *S]),
    ("aligned-first-read-pool", SIMDB, ["aligned", "first_read", "--per", "node", "--nth", "0", "--pool",
                                        "--field", "lag_ms", "--after", "5", "--cell", "star-5"]),
    ("aligned-parent-departed", SIMDB, ["aligned", "parent_departed", "--detail", "abrupt*", "--field", "seed_meter",
                                    "--bin", "500", "--before", "2", "--after", "6"]),
    ("aligned-node", SIMDB, ["aligned", "parent_departed", "--node", "3", "--field", "viewer_timeouts",
                             "--bin", "1000", "--before", "1", "--after", "3", "--json"]),
    ("aligned-refused", SIMDB, ["aligned", "onset", "--per", "node", "--field", "lag_p99", *S]),
    ("events", SIMDB, ["events"]),
    ("events-json", SIMDB, ["events", "--json", "--cell", "halve-5*"]),
    ("osc", SIMDB, ["osc"]),
    ("osc-json", SIMDB, ["osc", "--json", "--arm", "ppspp-*"]),
    ("rows", SIMDB, ["rows", *S]),
    ("rows-fields", SIMDB, ["rows", "--fields", "lag_p99_ms,unhealthy,collapsed,trace", *S, "--src",
                            "farm/sf0001*/i-1/*"]),
    ("rows-json", SIMDB, ["rows", "--json", "--cell", "star-5@sf0001"]),
    ("forecast", SIMDB, ["forecast"]),
    ("forecast-json", SIMDB, ["forecast", "--json"]),
    ("pull-dry-run", SIMDB, ["pull", "--run", FARM.split("/")[1], "--reads", "--dry-run"]),
    ("pull-dry-run-cell", SIMDB, ["pull", "--run", FARM.split("/")[1], "--reads", "--dry-run", "--cell", "star-5*",
                                  "--seed", "2002"]),
    ("pull-refused", SIMDB, ["pull", "--run", FARM.split("/")[1], "--dry-run"]),
    ("fold", SIMDB, ["fold", "<ROOT>/scratch.duckdb", "<JOBS>/00", "<JOBS>/02", "--src", "manual/x"]),
    ("export", SIMDB, ["export", "<ROOT>/scratch.duckdb", "<ROOT>/scratch-export"]),
    ("lake-missing", SIMDB, ["lake", "SELECT 1"]),
    ("index", SIMDB, ["index", "--quiet", "--workers", "2"]),
    ("lake-experiments", SIMDB, ["lake", "SELECT exp_id, kind, cells, jobs, arm_labels, base, question "
                                         "FROM experiments ORDER BY ALL"]),
    ("lake-jobs-json", SIMDB, ["lake", "SELECT k, j.sim, j.seed, e.cell, src FROM jobs j JOIN experiment_jobs e "
                                       "USING (k, src) ORDER BY ALL", "--json"]),
    # event_kinds holds every kind of the vocabulary, more than the dump
    # shows whole: the kinds the runs wrote.
    ("lake-event-kinds", SIMDB, ["lake", "SELECT kind, sim, class, value_meaning, documented, n FROM event_kinds "
                                         "WHERE n > 0 ORDER BY sim, kind"]),
    ("collapse", ANALYZE, ["collapse", *FARM_MATRICES]),
    ("series", ANALYZE, ["series", FARM_MATRICES[0], "--every", "5"]),
    ("series-where", ANALYZE, ["series", *FARM_MATRICES, "--where", "EXP=a", "--every", "10"]),
    ("map", ANALYZE, ["map", *FARM_MATRICES]),
    ("map-group", ANALYZE, ["map", *FARM_MATRICES, "--group", "EXP"]),
    ("pair", ANALYZE, ["pair", FARM_MATRICES[0], "--a", "SIM_EXP=off", "--b", "SIM_EXP=a",
                       "--ignore", "SIM_SEED"]),
    ("track", ANALYZE, ["track", *FARM_MATRICES]),
    ("track-runs", ANALYZE, ["track", FARM_MATRICES[1], "--runs"]),
    ("delays", ANALYZE, ["delays", *FARM_MATRICES]),
    # A selection whose reads a flush lacks pulls them (the aws stand-in
    # refuses), writing the S3 ledger: after the index, which reads it.
    ("pooled-viewer-json", SIMDB, ["pooled", "--by", "viewer", "--json", "--cell", "star-5"]),
]
# An export's fields that differ run to run.
VOLATILE = re.compile(r'("(?:at|sha256|bytes)": )(?:"[^"]*"|-?[0-9.e+]+)')
# The catalog's columns that differ run to run (its own times, its links'
# file identities), left out of its dump; and catalog_info's keys alike.
VOLATILE_COLS = ("indexed_at", "written_at", "mtime_ns", "inode", "bytes", "size")
VOLATILE_KEYS = ("indexed_at",)
# Its tables of what it read of the checkout's code, which moves as the
# code does; and catalog_info's keys alike: docs_version hashes the sims'
# rustdoc, which moves with any doc comment of theirs (the doc tables'
# fingerprints hold what reaches the catalog).
CODE_TABLES = ("raw.inputs",)
CODE_KEYS = ("docs_version",)
# A table with at most this many rows is dumped whole.
SMALL = 40


def environment(root):
    env = {k: v for k, v in os.environ.items() if k not in ("SIM_DB_ROOT", "GIT_DIR", "SIM_DUCKDB", "SIMLAB_UV")}
    return env | {"SIM_DB_ROOT": str(root / "target" / "sim-db"), "GIT_DIR": str(root / ".git"), "TZ": "UTC",
                  "COLUMNS": "100", "PYTHONHASHSEED": "0", "PATH": f"{root / 'bin'}:{os.environ.get('PATH', '')}"}


def normal(text, root):
    text = text.replace(str(root), "<ROOT>").replace(str(HACK.parent), "<REPO>").replace(str(Path.home()), "<HOME>")
    return re.sub(r"\.tmp\d+", ".tmp<PID>", text)


def invoke(root, jobs, script, args):
    args = [a.replace("<ROOT>", str(root)).replace("<JOBS>", str(jobs)) for a in args]
    p = subprocess.run([sys.executable, str(script), *args], cwd=root, env=environment(root), capture_output=True,
                       text=True, timeout=600)
    out, err = p.stdout, p.stderr
    # A traceback's frames move with the code: its exception is what failed.
    err = re.sub(r"Traceback \(most recent call last\):\n(?:[ \t].*\n|\n)*", "Traceback: ", err)
    if err:
        out += "--- stderr\n" + err
    return normal(out + f"--- exit {p.returncode}\n", root)


def masked(text):
    return VOLATILE.sub(r"\1<VOLATILE>", text)


def lake_json(root, jobs, sql):
    """A `lake --json` query's rows, and its output as text."""
    text = invoke(root, jobs, SIMDB, ["lake", sql, "--json"])
    first = text.split("\n", 1)[0]
    return (json.loads(first) if first.startswith("[") else []), text


def catalog_dump(root, jobs):
    """Every catalog table and view: its columns and types, its rows'
    count and a fingerprint of them (their text in order, md5), and a
    small one's rows, as `lake --json` gives them; volatile columns and
    keys (VOLATILE_COLS, VOLATILE_KEYS) and the code's (CODE_TABLES,
    CODE_KEYS) left out, the fixture's paths named as normal() names
    them, a link's inode in its name as <N>."""
    cols, text = lake_json(root, jobs, "SELECT table_schema AS s, table_name AS t, column_name AS c, data_type AS ty "
                                       "FROM information_schema.columns WHERE table_catalog = 'lake' "
                                       "ORDER BY s, t, ordinal_position")
    if not cols:
        return text
    tables = {}
    for r in cols:
        name = r["t"] if r["s"] == "main" else f"{r['s']}.{r['t']}"
        tables.setdefault(name, []).append((r["c"], r["ty"]))
    out = ["".join(f"{t}: {', '.join(f'{c} {ty}' for c, ty in cs)}\n" for t, cs in tables.items())]

    def body(t, cs):
        names = [c for c, _ in cs]
        keep = [c for c in names if c not in VOLATILE_COLS]
        where = (f" WHERE key NOT IN ({', '.join(repr(k) for k in (*VOLATILE_KEYS, *CODE_KEYS))})"
                 if {"key", "value"} <= set(names) else "")
        return f"(SELECT {', '.join(map(ident, keep))} FROM {t}{where})"

    def text_of(expr):
        out = f"replace(replace({expr}, {sql_str(str(root))}, '@ROOT@'), {sql_str(str(HACK.parent))}, '@REPO@')"
        return f"regexp_replace({out}, '[.][0-9]+[.]parquet', '.<N>.parquet', 'g')"

    shown = {t: cs for t, cs in tables.items() if t not in CODE_TABLES}
    union = " UNION ALL ".join(
        f"SELECT {sql_str(t)} AS t, count(*) AS n, md5(coalesce(string_agg({text_of('r::VARCHAR')}, chr(10) "
        f"ORDER BY {text_of('r::VARCHAR')}), '')) AS fingerprint FROM {body(t, cs)} r"
        for t, cs in shown.items())
    counts, text = lake_json(root, jobs, union + " ORDER BY t")
    out.append(text)
    for r in counts:
        if 0 < r["n"] <= SMALL:
            t = r["t"]
            _, text = lake_json(root, jobs, f"SELECT * FROM {body(t, shown[t])} ORDER BY ALL")
            out.append(f"=== {t}\n{re.sub(r'[.][0-9]+[.]parquet', '.<N>.parquet', text)}")
    return "".join(out)


def ident(s):
    return '"' + s.replace('"', '""') + '"'


def sql_str(s):
    return "'" + s.replace("'", "''") + "'"


def outputs(keep=None):
    """{name: text} of every invocation over a fresh fixture."""
    with tempfile.TemporaryDirectory(prefix="simlab-golden-") as tmp:
        root = Path(keep or tmp).resolve()
        if keep:
            shutil.rmtree(root, ignore_errors=True)
            root.mkdir(parents=True)
        saved = os.environ.get("SIM_DB_ROOT")
        try:
            jobs = build(root)
        finally:
            os.environ.pop("SIM_DB_ROOT", None)
            if saved is not None:
                os.environ["SIM_DB_ROOT"] = saved
        got = {}
        for name, script, args in INVOCATIONS:
            text = invoke(root, jobs, script, args)
            if name == "export":
                text = masked(text)
            got[name] = text
            if name == "index":
                got["catalog"] = catalog_dump(root, jobs)
        return got


def check(keep=None):
    """Whether today's outputs are the golden's: prints each difference."""
    return goldens.compare("golden_cli", goldens.read(GOLDEN), outputs(keep))


def capture(keep=None):
    goldens.write(GOLDEN, outputs(keep))
    print(f"wrote {GOLDEN}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=("check", "capture"))
    ap.add_argument("--keep", help="build the fixture in this dir and leave it")
    a = ap.parse_args()
    if a.cmd == "capture":
        capture(a.keep)
        return
    sys.exit(0 if check(a.keep) else 1)


if __name__ == "__main__":
    main()
