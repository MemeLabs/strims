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
    hack/testbed/benchsuite.py bursts ARM ... [--root DIR] [--repeat R]

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
repeats it ran in. `phases` prints the data links' one-way delays and probe
round trips (p50/p90), and the seed's samples of them (p50/max): a data
channel's buffered amount and a TCP socket's window (the window model's data
channel has both), between the given times (default: each 8 s). `bursts` prints, unfloored, what a shaper's
bucket moves first: each data link's bursts' first record's one-way delay
and their arrival spread (first record read to last), p50/p90, the testbed
against the arms (with --repeat all, each repeat's).
Comparisons take repeat R (default r1) and the cases that ran. They
read each testbed run's uplink queue and qdisc tree through bench.py,
which needs a release testbed_tables (TABLES_BIN, else `cargo build
--release -p strims-sim --example testbed_tables`).
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
        os.makedirs(os.path.join(root, f"r{k}"), exist_ok=True)
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


def cmd_bursts(args):
    root = flag(args, "--root", os.path.join(ROOT, "target", "bench", "runs"))
    repeat = flag(args, "--repeat", "r1")
    arms = args
    reps = repeats(root) if repeat == "all" else [repeat]
    print("columns per metric (ms, p50/p90): testbed " + " ".join(arms))
    print(f"{'case':13} {'rep':4}" + "".join(f" | {c:^{12 * (len(arms) + 1)}}" for c in ("first", "spread")))
    for r in reps:
        for c in ran(root, r):
            run = os.path.join(root, r, c)
            tb, arm_ms = None, []
            for a in arms:
                t, m = stats(run, a)
                tb = tb or t
                arm_ms.append(m)
            line = f"{c:13} {r:4}"
            for name in ("first", "spread"):
                cell = [show(pick(tb, data, name))] + [show(pick(m, data, name)) for m in arm_ms]
                line += " | " + " ".join(f"{x:>11}" for x in cell)
            print(line)


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
    got = {n: bench.phases(bench.load(d), edges, links) for n, d in dirs}
    q = lambda xs, p: bench.quantiles(xs, (p,))[0] if xs else None
    cols = (("owd", "owd p50/p90", .9), ("rtt", "rtt p50/p90", .9),
            ("buffered", "buffered p50/max", 1), ("cwnd", "cwnd p50/max", 1))
    width = sum(len(h) + 1 for _, h, _ in cols) - 1
    print(f"{'phase s':9}" + "".join(f" | {n:^{width}}" for n, _ in dirs))
    print(f"{'':9}" + (" | " + " ".join(h for _, h, _ in cols)) * len(dirs))
    for k, (lo, hi, _) in enumerate(got["testbed"]):
        row = f"{lo:>3.0f}-{hi:<4.0f} "
        for n, _ in dirs:
            m = got[n][k][2]
            row += " | " + " ".join(f"{show([q(m[c], .5), q(m[c], top)]):>{len(h)}}" for c, h, top in cols)
        print(row)


def main():
    cmds = {"run": cmd_run, "sim": cmd_sim, "summary": cmd_summary, "score": cmd_score,
            "phases": cmd_phases, "bursts": cmd_bursts}
    if len(sys.argv) < 2 or sys.argv[1] not in cmds:
        sys.exit(__doc__)
    cmds[sys.argv[1]](sys.argv[2:])


if __name__ == "__main__":
    main()
