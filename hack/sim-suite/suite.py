#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""A fast, directional check of a change against a base commit: the
working tree's sims against the base's, cell by cell, paired by seed.

  hack/sim-suite/suite.py run [--base REF] [--tier smoke|peers|scale]
                              [--seeds N] [--slots N] [--mem-gib N]
                              [--only CELL,...] [--rebuild] [--all]
  hack/sim-suite/suite.py calibrate [--base REF] [--tier ...] [--seeds N]

`run` builds the ppspp sim and strims-sim's scenarios from the working
tree (the checkout this script is in, changes and all) and from `--base`
(default `main`, a commit checked out in a worktree under
target/sim-suite/src), runs every cell of the tier at seeds 1..N under each,
and prints a scorecard. The base's rows are kept per commit
(target/sim-suite/rows/<commit>.jsonl), so later runs against the same base
run only the working tree. Rows are the sims' `ROW` objects with each run's
wall time and peak memory; runs are deterministic, so a cell and seed
always give the same row.

Tiers:

  smoke   a handful of short cells, one seed: does anything break.
  peers   the directional tier (about 5 minutes on 16 cores, fewer once the
          base is cached): large swarms, an overloaded one, greedy peers,
          WAN mixes, pull-heavy seeds, and the real stack (strims-sim).
  scale   1000 viewers, opt-in: memory and depth at scale.

The scorecard, per cell, shows each metric's base and working-tree medians
and the paired effect over seeds: the geometric mean ratio of B to A for
rates and times, the mean difference for shares, with a 95 % t-interval. A metric moved when the interval excludes no change and the
effect passes its tie band; it is then marked better (+) or worse (-) by
the metric's direction, or changed (~) where no direction is better.
Hard failures (a run without a row, a panic, memory past the cell's
budget) are listed first. Four seeds show direction, not effect sizes:
take what moves to the farm. Cheap cells run more seeds.

`calibrate` scores the base against itself at other seeds (seeds 1..N
against N+1..2N, paired in order): what moves there is seed noise, an
upper bound on what a change that only perturbs the timeline shows, and
a metric that moves in it needs a wider tie band.
"""

import argparse
import collections
import concurrent.futures as cf
import hashlib
import importlib.util
import json
import math
import os
import random
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
# The test binaries: (package, test target, test the jobs run).
BINARIES = {PPSPP: ("strims-ppspp", "sim"), SIM: ("strims-sim", "scenarios")}

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


TIERS = {
    "smoke": [
        cell("large", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "200", "SIM_MBIT": "2", "SIM_SECS": "10"}, 2,
             "a large swarm runs"),
        cell("wan-global", PPSPP, "sim_job", {**PP_AB, "SIM_SCENARIO": "WanGlobal", "SIM_SECS": "20"}, 1,
             "relays on residential uplinks"),
        cell("star-15", SIM, "calibration_star", {**SS_STAR, "STRIMS_SIM_CAL_VIEWERS": "15",
             "STRIMS_SIM_CAL_SEED_UP_MBIT": "10", "STRIMS_SIM_CAL_SECS": "20"}, 1, "the real stack runs"),
        cell("capped-hybrid", SIM, "capped_seed_hybrid", SS_BASE, 1, "pull behind a capped seed"),
    ],
    "peers": [
        cell("large-2mbit", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "500", "SIM_MBIT": "2", "SIM_SECS": "30"}, 5,
             "500 viewers of a cable seed: depth, fan-out, relay use at scale"),
        cell("large-4mbit", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "200", "SIM_MBIT": "4", "SIM_SECS": "30"}, 4,
             "200 viewers at a higher rate"),
        cell("overload", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "100", "SIM_MBIT": "6", "SIM_SECS": "30"}, 3,
             "100 viewers at 6 Mbit/s: trees fragment into cycles and pull-rooted chains"),
        cell("grabbers", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "6", "SIM_SECS": "30",
             "SIM_GREEDY_KINDS": "grab", "SIM_GREEDY_SHARE": "20"}, 6, "20 % grabbers: answer backlogs, memory"),
        cell("no-greedy", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "50", "SIM_MBIT": "6", "SIM_SECS": "30",
             "SIM_GREEDY": "0"}, 2, "the same swarm with honest viewers only"),
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


def build(src, dest):
    """Builds the test binaries at `src` and copies them into `dest`."""
    dest.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "CARGO_TARGET_DIR": str(CARGO), "CARGO_INCREMENTAL": "0"}
    for sim, (package, target) in BINARIES.items():
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


def base_binaries(sha, rebuild):
    """The base commit's binaries, built once in a worktree of their own."""
    dest = WORK / "bin" / sha
    if not rebuild and all((dest / sim).exists() for sim in BINARIES):
        return dest
    src = WORK / "src" / sha
    if not src.exists():
        git("worktree", "add", "--detach", str(src), sha)
    print(f"building base {sha[:8]}", file=sys.stderr)
    build(src, dest)
    return dest


def tree_binaries():
    """The working tree's binaries, under a label naming its commit and its
    changes."""
    head = git("rev-parse", "HEAD")
    diff = subprocess.run(["git", "diff", "HEAD"], cwd=ROOT, check=True, capture_output=True).stdout
    # New files under crates/ build too.
    for f in git("ls-files", "--others", "--exclude-standard", "crates").splitlines():
        diff += f.encode() + (ROOT / f).read_bytes()
    label = f"{head[:8]}+{hashlib.sha1(diff).hexdigest()[:8]}" if diff else head[:8]
    dest = WORK / "bin" / f"tree-{label}"
    print(f"building working tree {label}", file=sys.stderr)
    build(ROOT, dest)
    return label, dest


def job_id(label, c, seed):
    key = json.dumps([c["sim"], c["test"], sorted(c["env"].items()), seed])
    return f"{label}:{c['name']}:{seed}:{hashlib.sha1(key.encode()).hexdigest()[:12]}"


def run_jobs(jobs, slots, mem_gib):
    """Runs (label, bin_dir, cell, seed) jobs, the heaviest first, under
    `slots` processes and `mem_gib` of declared memory. Yields (job,
    row, error)."""
    lock = threading.Condition()
    used = [0.0]

    def one(job):
        _, bin_dir, c, seed = job
        need = min(c["mem_gib"], mem_gib)
        with lock:
            lock.wait_for(lambda: used[0] + need <= mem_gib)
            used[0] += need
        try:
            seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
            env = {**os.environ, **c["env"], seed_key: str(seed)}
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
        n = len({s for (name_, s) in a_rows if name_ == c["name"]} & {s for (name_, s) in b_rows if name_ == c["name"]})
        print(f"\n{c['name']}, {n} seeds ({c['why']})")
        print("\n".join(lines) if lines else "   (no rows)")
    print(f"\nmoved: {totals['+']} better, {totals['-']} worse, {totals['~']} changed; {totals[' ']} ties")


def run_cells(cells, runs, slots, mem_gib):
    """Runs every (label, bin_dir, store) at `seeds` for `cells`, skipping
    rows kept; returns each label's rows by (cell, seed) and the
    failures."""
    have = {label: load_rows(store) for label, _, store, _ in runs}
    jobs = [
        (label, bin_dir, c, s)
        for label, bin_dir, _, seeds in runs
        for c in cells
        for s in seeds(c)
        if job_id(label, c, s) not in have[label]
    ]
    failures = []
    t0 = time.monotonic()
    files = {label: open(store, "a") for label, _, store, _ in runs}
    for done, ((label, _, c, s), row, err) in enumerate(run_jobs(jobs, slots, mem_gib), 1):
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
            for s in seeds(c)
            if job_id(label, c, s) in have[label]
        }
        for label, _, _, seeds in runs
    }
    return rows, failures


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    cal = sub.add_parser("calibrate")
    cal.add_argument("--base", default="main")
    cal.add_argument("--tier", default="peers", choices=sorted(TIERS))
    cal.add_argument("--seeds", type=int, default=4)
    cal.add_argument("--slots", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    cal.add_argument("--mem-gib", type=float, default=40)
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
    a = ap.parse_args()

    cells = TIERS[a.tier]
    if getattr(a, "only", None):
        want = set(a.only.split(","))
        cells = [c for c in cells if c["name"] in want]
    count = lambda c: c["seeds"] or a.seeds
    seeds = lambda c: list(range(1, count(c) + 1))
    sha = git("rev-parse", a.base)
    base_bin = base_binaries(sha, a.rebuild)
    (WORK / "rows").mkdir(parents=True, exist_ok=True)
    base_store = WORK / "rows" / f"{sha}.jsonl"
    if a.cmd == "calibrate":
        both = lambda c: list(range(1, 2 * count(c) + 1))
        rows, failures = run_cells(cells, [(sha, base_bin, base_store, both)], a.slots, a.mem_gib)
        per = {c["name"]: count(c) for c in cells}
        b = {(c, s - per[c]): r for (c, s), r in rows[sha].items() if s > per[c]}
        a_rows = {k: r for k, r in rows[sha].items() if k[1] <= per[k[0]]}
        report = lambda: scorecard(cells, a_rows, b, f"{a.base} {sha[:8]} at seeds 1..N",
                                   "the same at seeds N+1..2N", True)
    else:
        tree_label, tree_bin = tree_binaries()
        tree_store = WORK / "rows" / f"tree-{tree_label}.jsonl"
        rows, failures = run_cells(
            cells, [(sha, base_bin, base_store, seeds), (tree_label, tree_bin, tree_store, seeds)],
            a.slots, a.mem_gib,
        )
        report = lambda: scorecard(cells, rows[sha], rows[tree_label], f"{a.base} {sha[:8]}",
                                   f"working tree {tree_label}", a.all)
    if failures:
        print("FAILURES")
        for f in failures:
            print("  " + f)
    report()
    if failures:
        sys.exit(1)


if __name__ == "__main__":
    main()
