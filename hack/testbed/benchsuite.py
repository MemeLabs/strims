#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The bare TCP bench's cases (hack/testbed/bench/*.sched) as a suite: runs
them on the testbed and in the sim, and compares the arms.

    hack/testbed/benchsuite.py run [--root DIR] [--repeats N] [CASE ...]
    hack/testbed/benchsuite.py sim ARM [--root DIR] [--bin BIN] [CASE ...]
                                   [-- testbed_bench args]
    hack/testbed/benchsuite.py summary ARM ... [--root DIR] [--repeat R]
    hack/testbed/benchsuite.py score ARM ... [--root DIR] [--repeat R]
    hack/testbed/benchsuite.py phases CASE ARM ... [--root DIR] [--repeat R]
                                      [--edges S,S,...] [--links I,I]

`run` runs each case on the testbed (bench.sh) into ROOT/r<k>/<case>, for
repeats 1..N (the default root is target/bench/runs); a case run already
there is kept. `sim` runs each case's testbed run (of every repeat) in the
sim under the window model into <run>/<ARM>, with the testbed_bench binary
BIN (default target/release/examples/testbed_bench) and its extra flags.
`summary` prints each case's drops, data links' and idle links' one-way
delays, probe round trips and uplink queue (p50/p90), the testbed against
the arms. `score` prints each arm's fit to the testbed: the mean
|log(arm / testbed)| of the data links' one-way delay and probe round trip
p50/p90, the uplink queue's p90 and the drops (each floored at 5), per case
and over the cases; with --repeat all, each case's fit averaged over the
repeats it ran in. `phases` prints the data links' one-way delays, probe
round trips and windows (a data channel's buffered amount) between the given
times (default: each 8 s).
Comparisons take repeat R (default r1) and the cases that ran.
"""

import math
import os
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import bench  # noqa: E402

ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
CASES = os.path.join(HERE, "bench")


def flag(args, name, default=None):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i : i + 2]
        return v
    return default


def all_cases():
    return sorted(f[:-6] for f in os.listdir(CASES) if f.endswith(".sched"))


def ran(root, repeat):
    d = os.path.join(root, repeat)
    if not os.path.isdir(d):
        return []
    return [c for c in all_cases() if os.path.exists(os.path.join(d, c, "node-0.jsonl"))]


def cmd_run(args):
    root = flag(args, "--root", os.path.join(ROOT, "target", "bench", "runs"))
    repeats = int(flag(args, "--repeats", "1"))
    env = {k: v for k, v in os.environ.items()
           if k not in ("VIEWERS", "SEED_UP_MBIT", "SECS", "SEED_UP_STEPS", "COMPETE", "BENCH_WEBRTC", "KINDS")}
    for k in range(1, repeats + 1):
        for c in args or all_cases():
            out = os.path.join(root, f"r{k}", c)
            if os.path.exists(os.path.join(out, "node-0.jsonl")):
                continue
            print(f"r{k} {c}", file=sys.stderr)
            with open(f"{out}.log", "w") as log:
                subprocess.run([os.path.join(HERE, "bench.sh"), out, os.path.join(CASES, f"{c}.sched")],
                               env=env, stdout=log, stderr=subprocess.STDOUT, check=False)


def cmd_sim(args):
    extra = []
    if "--" in args:
        i = args.index("--")
        extra = args[i + 1:]
        del args[i:]
    root = flag(args, "--root", os.path.join(ROOT, "target", "bench", "runs"))
    binary = flag(args, "--bin", os.path.join(ROOT, "target", "release", "examples", "testbed_bench"))
    arm, cases = args[0], args[1:]
    for r in sorted(os.listdir(root)):
        for c in cases or ran(root, r):
            run = os.path.join(root, r, c)
            if not os.path.isdir(run):
                continue
            p = subprocess.run([binary, "--sim", run, "--out", os.path.join(run, arm), "--window", *extra],
                               capture_output=True, text=True, check=False)
            if p.returncode != 0:
                print(f"{r} {c}: {p.stderr.strip()}", file=sys.stderr)


def stats(run, arm):
    """(testbed, arm) metrics of `run`."""
    _, ms = bench.compare(run, only=[arm])
    return ms[0], (ms[1] if len(ms) > 1 else {})


def pick(m, pred, name, qs=(0.5, 0.9)):
    for (cls, n), xs in sorted(m.items()):
        if n == name and pred(cls) and xs:
            if name == "drops":
                return [xs[0]]
            return [bench.quantiles(xs, qs)[i] for i in range(len(qs))]
    return None


def data(cls):
    return cls not in ("idle", "node 0") and not cls.startswith("128B")


def idle(cls):
    return cls.startswith("128B")


def node(cls):
    return cls == "node 0"


def show(vs):
    if not vs:
        return "-"
    return "/".join("-" if v is None else bench.fmt(v) for v in vs)


def cmd_summary(args):
    root = flag(args, "--root", os.path.join(ROOT, "target", "bench", "runs"))
    repeat = flag(args, "--repeat", "r1")
    arms = args
    cols = [("drops", node, "drops"), ("data owd", data, "owd"), ("idle owd", idle, "owd"),
            ("probe rtt", data, "rtt"), ("upq", node, "upq")]
    print(f"{repeat}: columns per metric: testbed " + " ".join(arms))
    print(f"{'case':13}" + "".join(f" | {c[0]:^{8 * (len(arms) + 1)}}" for c in cols))
    for c in ran(root, repeat):
        run = os.path.join(root, repeat, c)
        tb = None
        arm_ms = []
        for a in arms:
            t, m = stats(run, a)
            tb = tb or t
            arm_ms.append(m)
        line = f"{c:13}"
        for _, pred, name in cols:
            cell = [show(pick(tb, pred, name))] + [show(pick(m, pred, name)) for m in arm_ms]
            line += " | " + " ".join(f"{x:>7}" for x in cell)
        print(line)


def fit(tb, m):
    errs = []
    for pred, name in ((data, "owd"), (data, "rtt"), (node, "upq"), (node, "drops")):
        t, a = pick(tb, pred, name), pick(m, pred, name)
        if not t or not a:
            continue
        idx = [1] if name == "upq" else range(len(t))
        for i in idx:
            if t[i] is None or a[i] is None:
                continue
            errs.append(abs(math.log(max(a[i], 5.0) / max(t[i], 5.0))))
    return sum(errs) / len(errs) if errs else float("nan")


def repeats(root):
    return sorted((d for d in os.listdir(root) if d[:1] == "r" and d[1:].isdigit()),
                  key=lambda d: int(d[1:]))


def cmd_score(args):
    root = flag(args, "--root", os.path.join(ROOT, "target", "bench", "runs"))
    repeat = flag(args, "--repeat", "r1")
    arms = args
    reps = repeats(root) if repeat == "all" else [repeat]
    tot = {a: [] for a in arms}
    print(f"{'case':13}" + "".join(f"{a:>10}" for a in arms) + ("   repeats" if repeat == "all" else ""))
    cases = sorted({c for r in reps for c in ran(root, r)})
    for c in cases:
        line = f"{c:13}"
        runs = [os.path.join(root, r, c) for r in reps if c in ran(root, r)]
        for a in arms:
            es = [e for e in (fit(*stats(run, a)) for run in runs) if not math.isnan(e)]
            e = sum(es) / len(es) if es else float("nan")
            if not math.isnan(e):
                tot[a].append(e)
            line += f"{e:10.3f}"
        print(line + (f"{len(runs):10d}" if repeat == "all" else ""))
    print(f"{'mean':13}" + "".join(f"{sum(v) / len(v) if v else float('nan'):10.3f}" for v in tot.values()))


def cmd_phases(args):
    root = flag(args, "--root", os.path.join(ROOT, "target", "bench", "runs"))
    repeat = flag(args, "--repeat", "r1")
    edges = flag(args, "--edges")
    links = [int(x) for x in flag(args, "--links", "1,2").split(",")]
    case, arms = args[0], args[1:]
    run = os.path.join(root, repeat, case)
    if edges:
        edges = [float(x) for x in edges.split(",")]
    else:
        secs = int(bench.settings(run).get("SECS", "30"))
        edges = list(range(0, secs + 1, 8))
    dirs = [("testbed", run)] + [(a, os.path.join(run, a)) for a in arms]
    logs = {n: bench.load(d) for n, d in dirs}
    q = lambda xs, p: bench.quantiles(xs, (p,))[0] if xs else None
    print(f"{'phase s':9}" + "".join(f" | {n:^35}" for n, _ in dirs))
    print(f"{'':9}" + " | owd p50/p90  rtt p50/p90  cwnd p50/max" * len(dirs))
    for lo, hi in zip(edges, edges[1:]):
        row = f"{lo:>3.0f}-{hi:<4.0f} "
        for n, _ in dirs:
            d = logs[n]
            inside = lambda us: lo * 1e6 <= us < hi * 1e6
            owd = [(e["got"] - e["at"]) / 1000 for i in links for e in d.get(i, [])
                   if e.get("ev") == "r" and e["k"] == 0 and inside(e["at"])]
            rtt = [(e["got"] - e["at"]) / 1000 for e in d.get(0, [])
                   if e.get("ev") == "r" and e["k"] == 2 and e["from"] in links and inside(e["at"])]
            # A data channel's samples carry its buffered amount in place
            # of a window.
            cw = [e.get("cwnd", e.get("buffered", 0)) / 1024 for e in d.get(0, [])
                  if e.get("ev") == "s" and e["to"] in links and inside(e["t"])]
            row += (f" | {show([q(owd, .5), q(owd, .9)]):>11} {show([q(rtt, .5), q(rtt, .9)]):>11}"
                    f" {show([q(cw, .5), max(cw) if cw else None]):>11}")
        print(row)


def main():
    if len(sys.argv) < 2 or sys.argv[1] not in ("run", "sim", "summary", "score", "phases"):
        sys.exit(__doc__)
    {"run": cmd_run, "sim": cmd_sim, "summary": cmd_summary, "score": cmd_score,
     "phases": cmd_phases}[sys.argv[1]](sys.argv[2:])


if __name__ == "__main__":
    main()
