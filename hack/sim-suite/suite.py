#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""A fast, directional check of a change against a base commit: the
working tree's sims against the base's, cell by cell, paired by seed.

  hack/sim-suite/suite.py run [--base REF] [--tier smoke|peers|scale]
                              [--seeds N] [--slots N] [--mem-gib N]
                              [--only CELL,...] [--rebuild] [--all]
                              [--tree-env KEY=VALUE ...]
  hack/sim-suite/suite.py calibrate [--base REF] [--tier ...] [--seeds N]
                                    [--only CELL,...]
  hack/sim-suite/suite.py race --flags A,B,... [--knob SIM_EXP,STRIMS_SIM_EXP]
                               [--base REF] [--tier ...] [--keep N]
  hack/sim-suite/suite.py farm --sets "A,B;C" [--knob SIM_EXP,STRIMS_SIM_EXP]
                               [--ref REF] [--tier ...] [--seeds N]
                               [--first-seed N] [--farm-args ARGS] [--no-run]

`run` builds the ppspp sim and strims-sim's scenarios from the working
tree (the checkout this script is in, changes and all) and from `--base`
(default `main`, checked out in the suite's worktree,
target/sim-suite/src/base), runs every cell of the tier at seeds 1..N under each,
and prints a scorecard. Each sim's binary, and its rows, are kept by what
it is built from: crates/ and the workspace's Cargo and toolchain files,
less other packages' tests. A change to scripts, or to one sim's tests,
reuses the rest; later runs against the same base run only the working
tree, and a sim the working tree leaves as the base's isn't run twice.
Rows are the sims' `ROW` objects with each run's wall time and peak
memory; runs are deterministic, so a cell and seed always give the same
row.

Tiers:

  smoke   a handful of short cells, one seed: does anything break.
  peers   the directional tier (about 8 minutes on 16 cores, 5 once the
          base's rows are kept): large swarms, an overloaded one, churn,
          greedy peers, WAN mixes, pull-heavy seeds, and the real stack
          (strims-sim).
  scale   1000 viewers, opt-in: memory and depth at scale.

The scorecard, per cell, shows the seeds whose runs collapsed (8 unhealthy
seconds in a row, lasting past the first 15) under each, with a Fisher
exact test,
then each metric's base and working-tree medians
and the paired effect over seeds: the geometric mean ratio of B to A for
rates and times, the mean difference for shares, with a 95 % t-interval. A metric moved when the interval excludes no change and the
effect passes its tie band; it is then marked better (+) or worse (-) by
the metric's direction, or changed (~) where no direction is better.
Hard failures (a run without a row, a panic, memory past the cell's
budget) are listed first. Four seeds show direction, not effect sizes:
take what moves to the farm. Cheap cells run more seeds, and so do cells
that tip between healthy and collapsed by seed.

`race` finds the best combination of the working tree's flags (a knob
naming them comma-separated, by default SIM_EXP and STRIMS_SIM_EXP): round
0 runs a two-level fractional factorial over them (each flag's main effect
free of two-flag interactions) at half the cells' seeds, then each round
keeps the better half at more seeds (all of them, then twice), and the
last few are confirmed at fresh seeds (1001 on). An arm scores, against
the base on the same seeds, by its collapsed runs over all cells, then by
the geometric mean over cells of its lag p99 ratio; it is out if a cell's
delivery falls significantly, a cell collapses more often (Fisher p under
0.05: a sum over cells would let one cell's losses buy another's gains),
or a cell where the base never collapsed gets a lag p99 or startup p99
ratio whose interval lies past 1.10. Rounds print their tables, round 0
each flag's main effects, and the confirmation each cell's collapses and
lag p99 ratio.

`calibrate` scores the base against itself at other seeds (seeds 1..N
against N+1..2N, paired in order): what moves there is seed noise, an
upper bound on what a change that only perturbs the timeline shows, and
a metric that moves in it needs a wider tie band.

`farm` confirms flag sets on the sim farm (the main checkout's
hack/sim-farm, which keeps its config and state, also when this script
runs in a worktree). The farm builds commits, never the working tree, so
it runs one commit (`--ref`, default HEAD) with a matrix per cell whose
axes are the seeds (`--seeds` of them from `--first-seed`, every cell
alike) and the knob (the first of `--knob` for the ppspp sim, the second
for strims-sim) at `off` and at each set. Each set is scored against
`off`, which stands for the base: with its flags off the commit must
reproduce the base's traces. The cells needing more than 1 GiB a job go
in one farm run and the rest in another, side by side: the farm sizes
every slot of a run for its largest job. It prints each set's collapses
and lag p99 by cell, then its scorecard.
"""

import argparse
import collections
import itertools
import concurrent.futures as cf
import hashlib
import importlib.util
import json
import math
import os
import random
import shlex
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "target" / "sim-suite"
CARGO = WORK / "cargo"
GiB = 1 << 30

PPSPP = "ppspp"
SIM = "strims-sim"
# The test binaries: (package, test target, the package's directory).
BINARIES = {
    PPSPP: ("strims-ppspp", "sim", "crates/strims-ppspp"),
    SIM: ("strims-sim", "scenarios", "crates/strims-sim"),
}

PP_BASE = {"SIM_STREAMS": "16", "SIM_CHUNK": "4096", "SIM_MUNRO": "32768", "SIM_SERIES": "1"}
PP_CABLE = {**PP_BASE, "SIM_SCENARIO": "cable-seed", "SIM_SEED_POLICY": "adaptive", "SIM_SEED_ACCESS": "cable"}
PP_AB = {**PP_BASE, "SIM_CASE": "AdaptiveQos"}
SS_BASE = {"STRIMS_SIM_JOB": "1", "STRIMS_SIM_TCP": "1", "STRIMS_SIM_SERIES": "1"}
SS_STAR = {**SS_BASE, "STRIMS_SIM_CAL_MESH": "full", "STRIMS_SIM_CHUNK": "4096", "STRIMS_SIM_MUNRO": "32768"}


def cell(name, sim, test, env, mem_gib, why, seeds=None):
    """A cell: `seeds` overrides the tier's seed count (cheap cells take
    more, for tighter intervals)."""
    return {"name": name, "sim": sim, "test": test, "env": env, "mem_gib": mem_gib, "why": why, "seeds": seeds}


CHEAP = 12
# Cells whose seeds split between healthy runs and collapsed ones: at 4
# seeds only a huge effect shows.
TIPPING = 8


TIERS = {
    "smoke": [
        cell("large", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "200", "SIM_MBIT": "2", "SIM_SECS": "10"}, 2,
             "a large swarm runs", 1),
        cell("wan-global", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "WanGlobal", "SIM_SECS": "20"}, 1,
             "relays on residential uplinks", 1),
        cell("star-15", SIM, "calibration_star", {**SS_STAR, "STRIMS_SIM_CAL_VIEWERS": "15",
             "STRIMS_SIM_CAL_SEED_UP_MBIT": "10", "STRIMS_SIM_CAL_SECS": "20"}, 1, "the real stack runs", 1),
        cell("capped-hybrid", SIM, "capped_seed_hybrid", SS_BASE, 1, "pull behind a capped seed", 1),
    ],
    "peers": [
        cell("large-2mbit", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "500", "SIM_MBIT": "2", "SIM_SECS": "30"}, 5,
             "500 viewers of a cable seed: depth, fan-out, relay use at scale"),
        cell("large-4mbit", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "200", "SIM_MBIT": "4", "SIM_SECS": "30"}, 4,
             "200 viewers at a higher rate", TIPPING),
        cell("overload", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "100", "SIM_MBIT": "6", "SIM_SECS": "30"}, 3,
             "100 viewers at 6 Mbit/s: trees fragment into cycles and pull-rooted chains", TIPPING),
        cell("grabbers", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "6", "SIM_SECS": "30",
             "SIM_GREEDY_KINDS": "grab", "SIM_GREEDY_SHARE": "20"}, 6, "20 % grabbers: answer backlogs, memory"),
        cell("no-greedy", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "6", "SIM_SECS": "30",
             "SIM_GREEDY": "0"}, 2, "the same swarm with honest viewers only"),
        cell("churn", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "100", "SIM_MBIT": "4", "SIM_SECS": "60",
             "SIM_CHURN": "120", "SIM_CHURN_AWAY": "5"}, 3,
             "100 viewers, 120 % leaving a minute, new ones in their places 5 s on: trees repair or tip",
             TIPPING),
        cell("wan-global", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "WanGlobal", "SIM_SECS": "60"}, 1,
             "24 viewers worldwide, 6 links each, a datacenter seed", CHEAP),
        cell("wan-mobile", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "WanMobile", "SIM_SECS": "60"}, 1,
             "mobile and cable viewers", CHEAP),
        cell("sparse-lossy", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "SparseLossy", "SIM_SECS": "60"}, 1,
             "30 viewers, 6 links each, 1 % loss: repair by pull", CHEAP),
        cell("capped-seed", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "CappedSeed", "SIM_SECS": "60"}, 1,
             "a capped seed: pull-heavy", CHEAP),
        cell("residential", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "WanResidentialSeed", "SIM_SECS": "60"}, 1,
             "a residential seed and its relays", CHEAP),
        cell("lock-in", SIM, "calibration_star", {**SS_STAR, "STRIMS_SIM_CAL_VIEWERS": "31",
             "STRIMS_SIM_CAL_SEED_UP_MBIT": "5", "STRIMS_SIM_CAL_SECS": "60"}, 1,
             "real stack: 31 viewers behind a 5 Mbit/s seed, the pull lock-in"),
        cell("star-15", SIM, "calibration_star", {**SS_STAR, "STRIMS_SIM_CAL_VIEWERS": "15",
             "STRIMS_SIM_CAL_SEED_UP_MBIT": "10", "STRIMS_SIM_CAL_SECS": "60"}, 1, "real stack: a healthy star", CHEAP),
        cell("slow-viewer", SIM, "wan_slow_viewer_adaptive_uncapped", SS_BASE, 1, "a viewer whose link blocks", CHEAP),
        cell("residential-uncapped", SIM, "wan_residential_seed_adaptive_uncapped", SS_BASE, 1,
             "a residential seed that doesn't know its uplink", CHEAP),
        cell("mobile-uncapped", SIM, "wan_mobile_viewers_adaptive_uncapped", SS_BASE, 1, "mobile viewers", CHEAP),
        cell("capped-hybrid", SIM, "capped_seed_hybrid", SS_BASE, 1, "pull behind a capped seed", CHEAP),
        cell("starved-hybrid", SIM, "seed_starved_hybrid", SS_BASE, 1, "a seed starved for a while", CHEAP),
        cell("viewer-churn", SIM, "viewer_churn_hybrid", SS_BASE, 1, "real stack: viewers crash and restart",
             CHEAP),
    ],
    "scale": [
        cell("scale-1000", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "1000", "SIM_MBIT": "2", "SIM_SECS": "30"}, 10,
             "1000 viewers"),
    ],
}

# Metrics: (name, how to read it from a row, better: "lower", "higher" or
# None, kind: "ratio" (geometric) or "diff" (absolute), tie band).
LOWER, HIGHER = "lower", "higher"


def depth_mean(r):
    """The mean hops to the seed over chains that reach it (ppspp sim)."""
    hops = [(k, r.get(f"depth_{k}")) for k in range(1, 7)]
    w = sum(v for _, v in hops if v)
    return sum(k * v for k, v in hops if v) / w if w else None


def unhealthy(r):
    s = r.get("series")
    if not s:
        return None
    try:
        return analyze().verdict(s)["unhealthy"]
    except Exception:
        return None


def share(num, den):
    return lambda r: (r.get(num, 0) or 0) / den(r) if den(r) else None


METRICS = [
    ("lag_p50_ms", lambda r: r.get("lag_p50_ms"), LOWER, "ratio", 0.05),
    ("lag_p99_ms", lambda r: r.get("lag_p99_ms"), LOWER, "ratio", 0.05),
    ("unhealthy", unhealthy, LOWER, "diff", 0.03),
    ("delivery", lambda r: r.get("delivery"), HIGHER, "diff", 0.002),
    ("timeouts", lambda r: r.get("timeouts"), LOWER, "ratio", 0.10),
    ("wasted/read", lambda r: (r.get("wasted") or 0) / r["chunks_read"] if r.get("chunks_read") else None,
     LOWER, "diff", 0.01),
    ("seed_answer_share", lambda r: r.get("seed_pull_share"), None, "diff", 0.02),
    ("viewer_util_p50", lambda r: r.get("viewer_util_p50"), None, "diff", 0.02),
    ("viewer_util_p90", lambda r: r.get("viewer_util_p90"), None, "diff", 0.02),
    ("relay_share", lambda r: r.get("relay_share"), None, "diff", 0.03),
    ("depth_mean", depth_mean, None, "ratio", 0.05),
    ("depth_cycle", lambda r: r.get("depth_cycle"), LOWER, "diff", 0.01),
    ("depth_pull_rooted", lambda r: r.get("depth_pull_rooted"), LOWER, "diff", 0.01),
    ("fanout_p50", lambda r: r.get("fanout_p50"), None, "ratio", 0.10),
    ("fanout_p90", lambda r: r.get("fanout_p90"), None, "ratio", 0.10),
    ("parent_changes_per_min", lambda r: r.get("parent_changes_per_min"), LOWER, "ratio", 0.10),
    ("relay_declines", share("relay_rejected", lambda r: (r.get("relay_rejected") or 0) + (r.get("relay_accepted") or 0)),
     None, "diff", 0.03),
    ("declines_idle", lambda r: r.get("declines_idle"), LOWER, "diff", 0.05),
    ("declined_wrongly", share("declined_wrongly", lambda r: r.get("declined") or 0), LOWER, "diff", 0.05),
    ("meter_relays_p50", lambda r: r.get("meter_relays_p50") or (r.get("meter_forwarders") or [None] * 3)[1],
     None, "ratio", 0.10),
    ("meter_others_p50", lambda r: r.get("meter_others_p50") or (r.get("meter_others") or [None] * 3)[1],
     None, "ratio", 0.10),
    ("holder_short", lambda r: r.get("holder_short"), LOWER, "diff", 0.03),
    ("join_p50_ms", lambda r: r.get("join_p50_ms"), LOWER, "ratio", 0.10),
    ("join_p90_ms", lambda r: r.get("join_p90_ms"), LOWER, "ratio", 0.10),
    ("startup_p99_ms", lambda r: r.get("startup_p99_ms"), LOWER, "ratio", 0.10),
    ("rss_mb", lambda r: r.get("_rss_mb"), LOWER, "diff", 100),
    ("wall_s", lambda r: r.get("_wall_s"), LOWER, "ratio", 0.20),
]

_analyze = None


def analyze():
    """hack/capture/analyze.py, for its per-second verdicts."""
    global _analyze
    if _analyze is None:
        spec = importlib.util.spec_from_file_location("analyze", ROOT / "hack" / "capture" / "analyze.py")
        _analyze = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_analyze)
    return _analyze


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


def build(src, sim, dest):
    """Builds `sim`'s test binary at `src` and copies it into `dest`."""
    dest.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "CARGO_TARGET_DIR": str(CARGO), "CARGO_INCREMENTAL": "0"}
    package, target, _ = BINARIES[sim]
    out = subprocess.run(
        ["cargo", "test", "--release", "--locked", "-p", package, "--test", target, "--no-run",
         "--message-format=json"],
        cwd=src, env=env, check=True, capture_output=True, text=True,
    ).stdout
    exe = None
    for line in out.splitlines():
        m = json.loads(line)
        if m.get("reason") == "compiler-artifact" and m.get("target", {}).get("name") == target and m.get("executable"):
            exe = m["executable"]
    if exe is None:
        sys.exit(f"no {target} binary from {src}")
    bin_path = dest / sim
    tmp = bin_path.with_suffix(".tmp")
    tmp.write_bytes(Path(exe).read_bytes())
    tmp.chmod(0o755)
    tmp.replace(bin_path)


# What the test binaries are built from.
BUILD_PATHS = ["crates", "Cargo.toml", "Cargo.lock", "rust-toolchain.toml"]


def builds_into(path, sim):
    """Whether a file under BUILD_PATHS goes into `sim`'s binary: every one
    but other packages' tests, so editing one sim keeps the other's rows."""
    parts = path.split("/")
    if len(parts) > 2 and parts[0] == "crates" and parts[2] == "tests":
        return "/".join(parts[:2]) == BINARIES[sim][2]
    return True


def build_key(sha, sim):
    """A key for `sim`'s binary at `sha`: its build inputs' hashes."""
    listing = git("ls-tree", "-r", sha, "--", *BUILD_PATHS)
    kept = [line for line in listing.splitlines() if builds_into(line.split("\t", 1)[1], sim)]
    return hashlib.sha1("\n".join(kept).encode()).hexdigest()[:12]


def base_binary(sha, sim, rebuild):
    """The base commit's `sim` binary: its label and directory, built once
    in a worktree of the commit."""
    label = f"{sim}-{build_key(sha, sim)}"
    dest = WORK / "bin" / label
    if not rebuild and (dest / sim).exists():
        return label, dest
    # One worktree, moved to each base: cargo's fingerprints hold paths, so
    # a worktree per commit would rebuild every crate.
    src = WORK / "src" / "base"
    if not src.exists():
        git("worktree", "add", "--detach", str(src), sha)
    git("checkout", "--detach", "--quiet", sha, cwd=src)
    print(f"building base {sha[:8]} {sim} ({label})", file=sys.stderr)
    build(src, sim, dest)
    return label, dest


def tree_binary(sim, base):
    """The working tree's `sim` binary under a label naming its commit's
    build inputs and its changes to them: the base's own (`base`, its
    label and directory) when they are the same."""
    names = [f for f in git("diff", "HEAD", "--name-only", "--", *BUILD_PATHS).splitlines()
             if builds_into(f, sim)]
    diff = b""
    if names:
        diff = subprocess.run(["git", "diff", "HEAD", "--", *names], cwd=ROOT, check=True,
                              capture_output=True).stdout
    # New files under crates/ build too.
    for f in git("ls-files", "--others", "--exclude-standard", "crates").splitlines():
        if builds_into(f, sim):
            diff += f.encode() + (ROOT / f).read_bytes()
    key = build_key("HEAD", sim)
    if not diff and base[0] == f"{sim}-{key}":
        return base
    label = f"tree-{sim}-{key}+{hashlib.sha1(diff).hexdigest()[:8]}" if diff else f"tree-{sim}-{key}"
    dest = WORK / "bin" / label
    if not (dest / sim).exists():
        print(f"building working tree {sim} ({label})", file=sys.stderr)
        build(ROOT, sim, dest)
    return label, dest


def job_id(label, c, seed):
    key = json.dumps([c["sim"], c["test"], sorted(c["env"].items()), seed])
    return f"{label}:{c['name']}:{seed}:{hashlib.sha1(key.encode()).hexdigest()[:12]}"


def run_jobs(jobs, slots, mem_gib, need_gib):
    """Runs (label, bin_dir, cell, seed) jobs, the heaviest first, under
    `slots` processes and `mem_gib` of memory, a job taking its cell's
    `need_gib`. Yields (job, row, error)."""
    lock = threading.Condition()
    used = [0.0]

    def one(job):
        _, bin_dir, c, seed, extra = job
        need = min(need_gib(c), mem_gib)
        with lock:
            lock.wait_for(lambda: used[0] + need <= mem_gib)
            used[0] += need
        try:
            seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
            env = {**os.environ, **c["env"], **extra, seed_key: str(seed)}
            t0 = time.monotonic()
            p = subprocess.Popen(
                [str(bin_dir / c["sim"]), "--exact", c["test"], "--include-ignored", "--nocapture",
                 "--test-threads", "1"],
                env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
            )
            chunks = []
            for chunk in iter(lambda: p.stdout.read(1 << 16), b""):
                chunks.append(chunk)
            _, status, usage = os.wait4(p.pid, 0)
            p.returncode = os.waitstatus_to_exitcode(status)
            out = b"".join(chunks).decode(errors="replace")
            wall = time.monotonic() - t0
            at = out.find("ROW {")
            if at < 0 or p.returncode != 0:
                tail = "\n".join(out.splitlines()[-6:])
                return job, None, f"exit {p.returncode}: {tail}"
            row = json.loads(out[at + 4:].split("\n", 1)[0])
            row["_wall_s"] = round(wall, 2)
            row["_rss_mb"] = round(usage.ru_maxrss / 1024)
            if usage.ru_maxrss * 1024 > c["mem_gib"] * GiB:
                return job, row, f"peak memory {usage.ru_maxrss // 1024} MiB past the cell's {c['mem_gib']} GiB"
            return job, row, None
        finally:
            with lock:
                used[0] -= need
                lock.notify_all()

    order = sorted(jobs, key=lambda j: -j[2]["mem_gib"])
    with cf.ThreadPoolExecutor(slots) as ex:
        futures = [ex.submit(one, j) for j in order]
        for f in cf.as_completed(futures):
            yield f.result()


def load_rows(path):
    rows = {}
    if path.exists():
        for line in path.read_text().splitlines():
            r = json.loads(line)
            rows[r["id"]] = r
    return rows


# Student's t at 97.5 % by degrees of freedom; 2.0 past the table.
T975 = {1: 12.71, 2: 4.30, 3: 3.18, 4: 2.78, 5: 2.57, 6: 2.45, 7: 2.36, 8: 2.31, 9: 2.26, 10: 2.23,
        11: 2.20, 12: 2.18, 13: 2.16, 14: 2.14, 15: 2.13, 16: 2.12, 20: 2.09, 30: 2.04}


def t_interval(v):
    """The mean of `v` and its 95 % t-interval (none below two values)."""
    n = len(v)
    mean = sum(v) / n
    if n < 2:
        return mean, -math.inf, math.inf
    sd = math.sqrt(sum((x - mean) ** 2 for x in v) / (n - 1))
    t = T975.get(n - 1) or min((t for d, t in T975.items() if d >= n - 1), default=2.0)
    half = t * sd / math.sqrt(n)
    return mean, mean - half, mean + half


def effect(kind, pairs):
    """The paired effect of B against A over seeds, with its 95 % interval:
    the geometric mean ratio, or the mean difference."""
    if kind == "ratio":
        v = [math.log(b / a) for a, b in pairs if a and b and a > 0 and b > 0]
        if not v:
            return None
        m, lo, hi = t_interval(v)
        return math.exp(m), math.exp(lo) if lo > -math.inf else 0.0, math.exp(hi) if hi < math.inf else math.inf
    v = [b - a for a, b in pairs if a is not None and b is not None]
    if not v:
        return None
    return t_interval(v)


# A run collapsed when this many seconds in a row are unhealthy and they
# last past its startup (the real stack's trees take about 10 s to form).
STARTUP_SECS = 15
COLLAPSED_SECS = 8


def collapsed(r):
    s = r.get("series")
    if not s:
        return None
    run = 0
    for k, h in enumerate(analyze().verdict(s)["healthy"]):
        run = 0 if h else run + 1
        if run >= COLLAPSED_SECS and k >= STARTUP_SECS:
            return True
    return False


def fisher(k, n, m, n2):
    """The two-sided Fisher exact p of k in n against m in n2."""
    total, hits = n + n2, k + m
    prob = lambda x: math.comb(n, x) * math.comb(n2, hits - x) / math.comb(total, hits)
    seen = prob(k)
    return min(1.0, sum(prob(x) for x in range(max(0, hits - n2), min(n, hits) + 1) if prob(x) <= seen * (1 + 1e-9)))


def collapses(c, a_rows, b_rows):
    """The seeds whose runs collapsed under A and under B, and how many
    seeds both have."""
    seeds = sorted({s for (name, s) in a_rows if name == c["name"]} & {s for (name, s) in b_rows if name == c["name"]})
    count = lambda rows: sum(1 for s in seeds if collapsed(rows[(c["name"], s)]))
    return count(a_rows), count(b_rows), len(seeds)


def mark(better, kind, tie, e):
    """+ better, - worse, ~ moved with no better direction, blank for a
    tie."""
    if e is None:
        return " "
    x, lo, hi = e
    if kind == "ratio":
        moved = (lo > 1 or hi < 1) and abs(math.log(x)) > math.log(1 + tie)
        up = x > 1
    else:
        moved = (lo > 0 or hi < 0) and abs(x) > tie
        up = x > 0
    if not moved:
        return " "
    if better is None:
        return "~"
    return "+" if (up == (better == HIGHER)) else "-"


def fmt(x):
    if x is None:
        return "-"
    if isinstance(x, float) and abs(x) < 10:
        return f"{x:.3g}"
    return f"{x:.0f}"


def scorecard(cells, a_rows, b_rows, a_label, b_label, show_all=False):
    print(f"A = {a_label}   B = {b_label}")
    print("legend: + better  - worse  ~ moved  (ratio B/A or difference B-A, 95 % t-interval over seeds)")
    totals = collections.Counter()
    for c in cells:
        lines = []
        for name, get, better, kind, tie in METRICS:
            pairs = []
            for s in sorted({s for (name_, s) in a_rows if name_ == c["name"]}):
                ra, rb = a_rows.get((c["name"], s)), b_rows.get((c["name"], s))
                if ra is None or rb is None:
                    continue
                va, vb = get(ra), get(rb)
                if va is None or vb is None:
                    continue
                pairs.append((va, vb))
            if not pairs:
                continue
            e = effect(kind, pairs)
            m = mark(better, kind, tie, e)
            totals[m] += 1
            key = name in ("lag_p50_ms", "lag_p99_ms", "unhealthy", "delivery")
            if m == " " and not key and not show_all:
                continue
            med = lambda xs: sorted(xs)[len(xs) // 2]
            if e is None:
                eff = "-"
            elif kind == "ratio":
                eff = f"x{e[0]:.2f} [{e[1]:.2f},{e[2]:.2f}]"
            else:
                eff = f"{e[0]:+.3f} [{e[1]:+.3f},{e[2]:+.3f}]"
            lines.append(f"   {m} {name:20} {fmt(med([p[0] for p in pairs])):>9} {fmt(med([p[1] for p in pairs])):>9}  {eff}")
        # Whether a run collapses is near all-or-nothing, and a change
        # redraws which seeds do: the counts, not the pairs, carry it.
        ka, kb, n = collapses(c, a_rows, b_rows)
        if ka or kb:
            p = fisher(ka, n, kb, n)
            m = " " if p >= 0.05 else ("+" if kb < ka else "-")
            totals[m] += 1
            lines.insert(0, f"   {m} {'collapsed':20} {ka:>7}/{n} {kb:>7}/{n}  Fisher p={p:.2f}")
        print(f"\n{c['name']}, {n} seeds ({c['why']})")
        print("\n".join(lines) if lines else "   (no rows)")
    print(f"\nmoved: {totals['+']} better, {totals['-']} worse, {totals['~']} changed; {totals[' ']} ties")


# --- race -------------------------------------------------------------------

def design(flags):
    """Round 0's arms: a two-level fractional factorial over `flags`, the
    first five crossed in full and each further one the product of three of
    them (resolution IV: main effects aren't aliased with two-flag
    interactions). The all-off arm, the base itself, is left out."""
    k = len(flags)
    n = min(k, 5)
    gens = list(itertools.combinations(range(n), 3))[: k - n]
    arms = []
    for i in range(2 ** n):
        bits = [(i >> j) & 1 for j in range(n)]
        bits += [bits[a] ^ bits[b] ^ bits[c] for a, b, c in gens]
        arm = tuple(f for f, b in zip(flags, bits) if b)
        if arm:
            arms.append(arm)
    return arms


GUARD_RATIO = 1.10


def arm_score(cells, base_rows, arm_rows):
    """An arm against the base on the same seeds: its collapsed runs and the
    base's, the geometric mean over cells of its lag p99 ratio, and the
    guardrails it broke."""
    ka_all = kb_all = 0
    logs = []
    broken = []
    cells_out = []
    get = {name: (fn, better, kind, tie) for name, fn, better, kind, tie in METRICS}
    for c in cells:
        ka, kb, n = collapses(c, base_rows, arm_rows)
        if n == 0:
            continue
        ka_all += ka
        kb_all += kb
        if kb > ka and fisher(ka, n, kb, n) < 0.05:
            broken.append(f"{c['name']} collapsed {ka}->{kb}/{n}")
        def pairs(name):
            fn = get[name][0]
            out = []
            for (cell, seed), ra in base_rows.items():
                rb = arm_rows.get((cell, seed))
                if cell != c["name"] or rb is None:
                    continue
                va, vb = fn(ra), fn(rb)
                if va is not None and vb is not None:
                    out.append((va, vb))
            return out
        e = effect("ratio", pairs("lag_p99_ms"))
        if e is not None:
            logs.append(math.log(e[0]))
        cells_out.append((c["name"], ka, kb, n, e))
        fn, better, kind, tie = get["delivery"]
        if mark(better, kind, tie, effect(kind, pairs("delivery"))) == "-":
            broken.append(f"{c['name']} delivery")
        if ka == 0:
            for name in ("lag_p99_ms", "startup_p99_ms"):
                e = effect("ratio", pairs(name))
                if e is not None and e[1] > GUARD_RATIO:
                    broken.append(f"{c['name']} {name} x{e[0]:.2f}")
    p99 = math.exp(sum(logs) / len(logs)) if logs else None
    return {"collapsed": kb_all, "base_collapsed": ka_all, "p99": p99, "broken": broken, "cells": cells_out}


def rank_key(sc):
    return (bool(sc["broken"]), sc["collapsed"], sc["p99"] if sc["p99"] is not None else math.inf)


def race(a, cells, count, sha, base, sims, union):
    flags = [f.strip() for f in a.flags.split(",") if f.strip()]
    knobs = [k.strip() for k in a.knob.split(",") if k.strip()]
    tree = {sim: tree_binary(sim, base[sim]) for sim in sims}

    def arm_runs(arm, seeds):
        env = {k: ",".join(arm) for k in knobs}
        tag = hashlib.sha1(json.dumps(sorted(env.items())).encode()).hexdigest()[:8]
        return {f"{label}~{tag}": (bin_dir, seeds, sim, env) for sim, (label, bin_dir) in tree.items()}

    def play(arms, seeds, title, per_cell=False):
        runs = {label: (bin_dir, seeds, sim, {}) for sim, (label, bin_dir) in base.items()}
        labels = {}
        for arm in arms:
            r = arm_runs(arm, seeds)
            labels[arm] = list(r)
            runs.update(r)
        rows, failures = run_cells(cells, runs, a.slots, a.mem_gib)
        base_rows = union(rows, [label for label, _ in base.values()])
        scores = {arm: arm_score(cells, base_rows, union(rows, labels[arm])) for arm in arms}
        print(f"\n{title}: {len(arms)} arms")
        print(f"   {'arm':58} collapsed (base)   p99 ratio   guardrails")
        for arm in sorted(arms, key=lambda x: rank_key(scores[x])):
            sc = scores[arm]
            p99 = "-" if sc["p99"] is None else f"x{sc['p99']:.3f}"
            print(f"   {','.join(arm):58} {sc['collapsed']:5} ({sc['base_collapsed']:3})   {p99:>9}   "
                  f"{'; '.join(sc['broken'][:3]) + (' ...' if len(sc['broken']) > 3 else '') or 'ok'}")
            for name, ka, kb, n, e in sc["cells"] if per_cell else ():
                p99 = "-" if e is None else f"x{e[0]:.2f} [{e[1]:.2f},{e[2]:.2f}]"
                print(f"      {name:22} collapsed {ka:>3} -> {kb:>3} of {n:<3}  lag p99 {p99}")
        for f in failures:
            print("   FAILURE " + f)
        sys.stdout.flush()
        return scores

    upto = lambda mult: (lambda c: list(range(1, max(2, math.ceil(count(c) * mult)) + 1)))
    arms = design(flags)
    scores = play(arms, upto(0.5), "round 0 (fractional factorial, half the seeds)")
    # Main effects: the mean with a flag on less the mean with it off.
    print("\nmain effects (round 0): log p99 ratio, collapsed runs, arms out")
    for f in flags:
        on = [scores[x] for x in arms if f in x]
        off = [scores[x] for x in arms if f not in x] + [{"collapsed": scores[arms[0]]["base_collapsed"], "p99": 1.0, "broken": []}]
        lp = lambda xs: sum(math.log(x["p99"]) for x in xs if x["p99"]) / max(1, sum(1 for x in xs if x["p99"]))
        col = lambda xs: sum(x["collapsed"] for x in xs) / max(1, len(xs))
        out = lambda xs: sum(1 for x in xs if x["broken"]) / max(1, len(xs))
        print(f"   {f:18} {lp(on) - lp(off):+.3f}   {col(on) - col(off):+6.2f}   {out(on):.0%} vs {out(off):.0%}")
    for mult in (1.0, 2.0):
        live = sorted(arms, key=lambda x: rank_key(scores[x]))
        live = [x for x in live if not scores[x]["broken"]] or live[:1]
        arms = live[: max(a.keep, math.ceil(len(live) / 2))]
        scores = play(arms, upto(mult), f"round at {mult:g}x the seeds")
    best = sorted(arms, key=lambda x: rank_key(scores[x]))[: a.keep]
    fresh = lambda c: list(range(1001, 1001 + max(2, count(c))))
    play(best, fresh, "confirmation at fresh seeds", per_cell=True)


def run_cells(cells, runs, slots, mem_gib):
    """Runs every binary of `runs` ({label: (bin_dir, seeds, sim, env)})
    at its seeds for the cells of its sim, with `env` added to every job's
    environment, skipping rows kept (per label, in
    target/sim-suite/rows/<label>.jsonl); returns each label's rows by
    (cell, seed) and the failures."""
    store = lambda label: WORK / "rows" / f"{label}.jsonl"
    have = {label: load_rows(store(label)) for label in runs}
    jobs = [
        (label, bin_dir, c, s, env)
        for label, (bin_dir, seeds, sim, env) in runs.items()
        for c in cells
        if c["sim"] == sim
        for s in seeds(c)
        if job_id(label, c, s) not in have[label]
    ]
    # A job takes its cell's peak memory in kept rows, a quarter over, and
    # the declared budget until the cell has rows: budgets are worst cases,
    # and holding to them left most cores idle while the heavy cells ran.
    peaks = {}
    for kept in have.values():
        for rec in kept.values():
            if rec["row"].get("_rss_mb"):
                peaks[rec["cell"]] = max(peaks.get(rec["cell"], 0), rec["row"]["_rss_mb"])
    need = lambda c: min(c["mem_gib"], 1.25 * peaks[c["name"]] / 1024) if c["name"] in peaks else c["mem_gib"]
    failures = []
    t0 = time.monotonic()
    files = {label: open(store(label), "a") for label in runs}
    for done, ((label, _, c, s, _), row, err) in enumerate(run_jobs(jobs, slots, mem_gib, need), 1):
        if err:
            failures.append(f"{label} {c['name']} seed {s}: {err}")
        if row is not None:
            rec = {"id": job_id(label, c, s), "cell": c["name"], "seed": s, "row": row}
            have[label][rec["id"]] = rec
            files[label].write(json.dumps(rec) + "\n")
            files[label].flush()
        print(f"\r{done}/{len(jobs)} runs, {time.monotonic() - t0:.0f} s", end="", file=sys.stderr)
    print(file=sys.stderr)
    for f in files.values():
        f.close()
    rows = {
        label: {
            (c["name"], s): have[label][job_id(label, c, s)]["row"]
            for c in cells
            if c["sim"] == sim
            for s in seeds(c)
            if job_id(label, c, s) in have[label]
        }
        for label, (_, seeds, sim, _) in runs.items()
    }
    return rows, failures


# --- farm -------------------------------------------------------------------

def farm_root():
    """The main checkout, whose hack/sim-farm keeps the farm's config and
    state."""
    return Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).parent


def peak_gib(sha):
    """Each cell's peak memory, in GiB, over the kept rows of binaries
    built from `sha`'s build inputs (with knobs, or changes on top)."""
    keys = [f"{sim}-{build_key(sha, sim)}" for sim in BINARIES]
    peaks = {}
    for path in (WORK / "rows").glob("*.jsonl"):
        if not any(path.stem in (k, f"tree-{k}") or path.stem.startswith((f"{k}~", f"tree-{k}~", f"tree-{k}+"))
                   for k in keys):
            continue
        for rec in load_rows(path).values():
            m = rec["row"].get("_rss_mb")
            if m:
                peaks[rec["cell"]] = max(peaks.get(rec["cell"], 0), m / 1024)
    return peaks


# Cells whose jobs need more than this go in a farm run of their own.
FARM_SPLIT_GIB = 1


def farm(a, cells):
    sha = git("rev-parse", a.ref)
    values = ["off"] + [x.strip() for x in a.sets.split(";") if x.strip()]
    knob = dict(zip((PPSPP, SIM), (k.strip() for k in a.knob.split(","))))
    seeds = [str(s) for s in range(a.first_seed, a.first_seed + a.seeds)]
    # Matrix names are the run's EC2 tag, joined: keep them short, and
    # name the inputs so a rerun adopts the run.
    key = json.dumps([sha, values, knob, seeds, [c["name"] for c in cells]])
    name = "s" + hashlib.sha1(key.encode()).hexdigest()[:5]
    peaks = peak_gib(sha)
    out = WORK / "farm"
    out.mkdir(parents=True, exist_ok=True)
    paths, mems = [], []
    for i, c in enumerate(cells):
        package, target, _ = BINARIES[c["sim"]]
        seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
        mem = c["mem_gib"]
        if c["name"] in peaks:
            mem = min(mem, math.ceil(1.25 * peaks[c["name"]] * 2) / 2)
        canary = {"SIM_SECS": "2"} if c["sim"] == PPSPP else {"STRIMS_SIM_STREAM_SECS": "5"}
        q = json.dumps
        text = "\n".join([
            f"# sim-suite farm {name}: {c['name']}, {c['why']}",
            f"package = {q(package)}",
            f"target = {q(target)}",
            f"test = {q(c['test'])}",
            f"slot_mem_gib = {mem}",
            "[arms]",
            f"x = {q(sha)}",
            "[env]",
            *(f"{k} = {q(v)}" for k, v in sorted(c["env"].items())),
            "[axes]",
            f"{knob[c['sim']]} = {q(values)}",
            f"{seed_key} = {q(seeds)}",
            "[canary]",
            *(f"{k} = {q(v)}" for k, v in canary.items()),
            "",
        ])
        path = out / f"{name}-{i:02d}.toml"
        path.write_text(text)
        paths.append(path)
        mems.append(mem)
    farm_py = farm_root() / "hack" / "sim-farm" / "farm.py"
    if not a.no_run:
        print(f"farm run {name}: {len(cells)} matrices, {len(values)} sets x {len(seeds)} seeds", file=sys.stderr)
        groups = [[p for p, m in zip(paths, mems) if (m > FARM_SPLIT_GIB) == heavy] for heavy in (True, False)]
        procs = [subprocess.Popen([sys.executable, str(farm_py), "run", *map(str, g), *shlex.split(a.farm_args)])
                 for g in groups if g]
        for p in procs:
            p.wait()
    rows = {v: {} for v in values}
    for i, c in enumerate(cells):
        seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
        state = farm_root() / "target" / "sim-farm" / f"{name}-{i:02d}" / "rows.jsonl"
        for rec in load_rows(state).values() if state.exists() else ():
            v = rec["env"].get(knob[c["sim"]])
            if v in rows:
                row = {**rec["row"], "_rss_mb": rec.get("max_rss_mb"), "_wall_s": rec.get("wall_s")}
                rows[v][(c["name"], int(rec["env"][seed_key]))] = row
    for v in values[1:]:
        sc = arm_score(cells, rows["off"], rows[v])
        p99 = "-" if sc["p99"] is None else f"x{sc['p99']:.3f}"
        print(f"\n{v} against off at {sha[:8]}: collapsed {sc['collapsed']} ({sc['base_collapsed']}), "
              f"p99 {p99}, guardrails {'; '.join(sc['broken']) or 'ok'}")
        for cname, ka, kb, n, e in sc["cells"]:
            p = "-" if e is None else f"x{e[0]:.2f} [{e[1]:.2f},{e[2]:.2f}]"
            print(f"   {cname:22} collapsed {ka:>3} -> {kb:>3} of {n:<3} Fisher p={fisher(ka, n, kb, n):.2f}  lag p99 {p}")
    for v in values[1:]:
        print()
        scorecard(cells, rows["off"], rows[v], f"{sha[:8]} off", f"{sha[:8]} {v}", a.all)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    cal = sub.add_parser("calibrate")
    cal.add_argument("--base", default="main")
    cal.add_argument("--tier", default="peers", choices=sorted(TIERS))
    cal.add_argument("--seeds", type=int, default=4)
    cal.add_argument("--slots", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    cal.add_argument("--mem-gib", type=float, default=40)
    cal.add_argument("--only", help="cells, comma-separated")
    cal.add_argument("--rebuild", action="store_true")
    r = sub.add_parser("run")
    r.add_argument("--base", default="main")
    r.add_argument("--tier", default="peers", choices=sorted(TIERS))
    r.add_argument("--seeds", type=int, default=4)
    r.add_argument("--slots", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    r.add_argument("--mem-gib", type=float, default=40)
    r.add_argument("--only", help="cells, comma-separated")
    r.add_argument("--rebuild", action="store_true", help="rebuild the base's binaries")
    r.add_argument("--all", action="store_true", help="every metric, ties too")
    r.add_argument("--tree-env", action="append", default=[], metavar="KEY=VALUE",
                   help="set for the working tree's runs only, e.g. a knob that turns a change on")
    rc = sub.add_parser("race")
    rc.add_argument("--flags", required=True, help="the flags to race, comma-separated")
    rc.add_argument("--knob", default="SIM_EXP,STRIMS_SIM_EXP",
                    help="the environment knobs that take the flags, comma-separated")
    rc.add_argument("--base", default="main")
    rc.add_argument("--tier", default="peers", choices=sorted(TIERS))
    rc.add_argument("--seeds", type=int, default=4)
    rc.add_argument("--slots", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    rc.add_argument("--mem-gib", type=float, default=40)
    rc.add_argument("--only", help="cells, comma-separated")
    rc.add_argument("--rebuild", action="store_true")
    rc.add_argument("--keep", type=int, default=3, help="arms the confirmation keeps")
    fc = sub.add_parser("farm")
    fc.add_argument("--sets", required=True, help="flag sets, ;-separated, each comma-separated")
    fc.add_argument("--knob", default="SIM_EXP,STRIMS_SIM_EXP",
                    help="the knobs that take a set: the ppspp sim's, then strims-sim's")
    fc.add_argument("--ref", default="HEAD", help="the commit the farm builds")
    fc.add_argument("--tier", default="peers", choices=sorted(TIERS))
    fc.add_argument("--seeds", type=int, default=32, help="seeds per cell, every cell alike")
    fc.add_argument("--first-seed", type=int, default=2001)
    fc.add_argument("--only", help="cells, comma-separated")
    fc.add_argument("--all", action="store_true", help="every metric, ties too")
    fc.add_argument("--farm-args", default="", help="more arguments for farm.py run")
    fc.add_argument("--no-run", action="store_true", help="report the rows the farm has")
    a = ap.parse_args()

    cells = TIERS[a.tier]
    if getattr(a, "only", None):
        want = set(a.only.split(","))
        cells = [c for c in cells if c["name"] in want]
    if a.cmd == "farm":
        farm(a, cells)
        return
    count = lambda c: c["seeds"] or a.seeds
    seeds = lambda c: list(range(1, count(c) + 1))
    sha = git("rev-parse", a.base)
    sims = sorted({c["sim"] for c in cells})
    base = {sim: base_binary(sha, sim, a.rebuild) for sim in sims}
    (WORK / "rows").mkdir(parents=True, exist_ok=True)
    union = lambda rows, labels: {k: r for label in labels for k, r in rows[label].items()}
    if a.cmd == "race":
        race(a, cells, count, sha, base, sims, union)
        return
    if a.cmd == "calibrate":
        both = lambda c: list(range(1, 2 * count(c) + 1))
        runs = {label: (bin_dir, both, sim, {}) for sim, (label, bin_dir) in base.items()}
        rows, failures = run_cells(cells, runs, a.slots, a.mem_gib)
        every = union(rows, runs)
        per = {c["name"]: count(c) for c in cells}
        b = {(c, s - per[c]): r for (c, s), r in every.items() if s > per[c]}
        a_rows = {k: r for k, r in every.items() if k[1] <= per[k[0]]}
        report = lambda: scorecard(cells, a_rows, b, f"{a.base} {sha[:8]} at seeds 1..N",
                                   "the same at seeds N+1..2N", True)
    else:
        tree = {sim: tree_binary(sim, base[sim]) for sim in sims}
        # The working tree's knobs name its rows apart from the same binary's
        # without them.
        env = dict(kv.split("=", 1) for kv in a.tree_env)
        if env:
            tag = hashlib.sha1(json.dumps(sorted(env.items())).encode()).hexdigest()[:8]
            tree = {sim: (f"{label}~{tag}", bin_dir) for sim, (label, bin_dir) in tree.items()}
        runs = {label: (bin_dir, seeds, sim, {}) for sim, (label, bin_dir) in base.items()}
        runs.update({label: (bin_dir, seeds, sim, env) for sim, (label, bin_dir) in tree.items()})
        rows, failures = run_cells(cells, runs, a.slots, a.mem_gib)
        a_rows = union(rows, [label for label, _ in base.values()])
        b_rows = union(rows, [label for label, _ in tree.values()])
        tree_labels = ", ".join(label for label, _ in tree.values())
        knobs = " ".join(a.tree_env)
        report = lambda: scorecard(cells, a_rows, b_rows, f"{a.base} {sha[:8]}",
                                   f"working tree ({tree_labels}){' with ' + knobs if knobs else ''}", a.all)
    if failures:
        print("FAILURES")
        for f in failures:
            print("  " + f)
    report()
    if failures:
        sys.exit(1)


if __name__ == "__main__":
    main()
