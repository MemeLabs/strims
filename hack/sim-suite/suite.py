#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""A fast, directional check of a change against a base commit: the
working tree's sims against the base's, cell by cell, paired by seed.

  hack/sim-suite/suite.py run [--base REF] [--tier smoke|peers|scale]
                              [--seeds N] [--slots N] [--mem-gib N]
                              [--only CELL,...] [--rebuild] [--all]
                              [--tree-env KEY=VALUE ...] [--env KEY=VALUE ...]
  hack/sim-suite/suite.py calibrate [--base REF] [--tier ...] [--seeds N]
                                    [--only CELL,...] [--env KEY=VALUE ...]
  hack/sim-suite/suite.py race --flags A,B,... [--knob SIM_EXP,STRIMS_SIM_EXP]
                               [--base REF] [--tier ...] [--keep N]
                               [--env KEY=VALUE ...]
  hack/sim-suite/suite.py farm (--sets "A,B;C" | --refs a=REF,b=REF) [--knob SIM_EXP,STRIMS_SIM_EXP]
                               [--ref REF] [--tier ...] [--seeds N]
                               [--first-seed N] [--against SET] [--question TEXT]
                               [--farm-args ARGS] [--no-run] [--env KEY=VALUE ...]
  hack/sim-suite/suite.py score RUN:ARM RUN:ARM [--tier ...] [--only CELL,...]
                                [--knob SIM_EXP,STRIMS_SIM_EXP] [--all]
  hack/sim-suite/suite.py list [--tier ...] [--only CELL,...]

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
row. Each run also writes its tables (hack/sim-db: its seconds, every
read, finer bins), which fold into a store of the invocation's and go to
the cache every worktree shares (the main checkout's target/sim-db),
where `simdb.py` queries them; a run's line in target/sim-suite/rows/
names its source there, and the verdicts read its series from them. A
run whose tables have left the cache runs again. `--env` sets a variable
for every arm's runs as part of each cell, so the rows of one setting
never stand for another's.

Tiers:

  smoke   a handful of short cells, one seed: does anything break.
  robust  the real stack across shapes (stars of 3-31 viewers behind 5-20
          Mbit/s seeds, halvings, sparse meshes, jitter, rate limits, thin
          relays, every WAN and fault case), for a change that must hold in
          general: run it with --seeds 8 or more.
  peers   the directional tier (about 8 minutes on 16 cores, 5 once the
          base's rows are kept): large swarms, an overloaded one, churn,
          greedy peers, WAN mixes, pull-heavy seeds, and the real stack
          (strims-sim).
  scale   1000 viewers, opt-in: memory and depth at scale.
  browsers
          robust's cells under the window model with every host a browser
          (STRIMS_SIM_RTC=BROWSERS): WebRTC links on the model's data
          channels, three hosts in four on Chromium's SCTP and one on
          Firefox's. Its cells keep robust's names, so `score` pairs a
          browsers run with a robust one.
  startup today's cells with every viewer starting with the seed, as the
          cells the audience default (below) moved ran before: the
          near-pathological start, as an explicit stress test, under the
          names and settings those cells had, so their rows still pair.

The audience: a cell's viewers trickle in (`SIM_ARRIVALS`,
`STRIMS_SIM_ARRIVALS` = trickle: all of them within the stream's first
third, at random) where its scripted events come after they're in
(TRICKLE), as a broadcast's audience does; the collapse reproducers, the
fixed-fault cells and churn keep every viewer at the start (t0), and the
sims' tests keep it by default (the testbed twin, CI and the sweeps). The
audience cells (`audience_cells`, in peers, robust, browsers and net) add
a scheduled event's burst into a warm swarm, an event's end (60 % leaving
within 10 s, gracefully and abruptly, as separate cells) and the two
together; strims-sim's run on stars and organic networks, where those
leaving can't strand the rest. The verdicts count each viewer from its own
join (analyze.py), so trickle cells need the stream to run past the last
arrival and its grace: the ppspp sim's 30 s cells run 45 s.

The scorecard, per cell, shows the seeds whose runs collapsed (8 unhealthy
seconds in a row, lasting past the first 15 when viewers start with the
seed) under each, with a Fisher exact test,
then each metric's base and working-tree medians
and the paired effect over seeds: the geometric mean ratio of B to A for
rates and times, the mean difference for shares, with a 95 % t-interval. A metric moved when the interval excludes no change and the
effect passes its tie band; it is then marked better (+) or worse (-) by
the metric's direction, or changed (~) where no direction is better.
Wall time and memory move with whatever else loads the machine: they are
marked h when they move and left out of the totals.
Last, how many runs give
the same trace as A's at the same cell and seed, and the same reads where
both rows carry their digest (`reads_hash`: the trace covers what the nodes
did, so a change that moves only readers keeps it): all of them for a
change that leaves the sims' behaviour alone, and which cells differ when
only some do. Hard failures (a run without a row, a panic, memory past the
cell's budget) are listed first. Four seeds show direction, not effect sizes:
take what moves to the farm. Cheap cells run more seeds, and so do cells
that tip between healthy and collapsed by seed.

The sims' results catalog (hack/simlab/catalog.py) computes every score
here: the runs' tables go to the cache, the catalog indexes them (each
run's row and analyze.py's verdict), and its scoring macros
(hack/simlab/sql/scoring.sql, over METRICS) pair two arms' runs and give the
effects, collapse tests, traces and guardrails this script prints. A
session loads the macros as this script defines them, so a branch's
METRICS score its runs. golden.py checks the macros against the Python
scoring suite.py had at golden.py's REFERENCE.

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
a cell where the base never collapsed gets a lag p99, weak viewers' lag
p99, join p90 or caught-up p90 ratio (the viewers' own joins, with
arrivals) whose interval lies past 1.10, or defecting comes to pay more
(`defector_gain`). Rounds print their tables, round 0
each flag's main effects, and the confirmation each cell's collapses and
lag p99 ratio.

`--rank overhead` (race, farm, score) spends latency on bytes: arms rank
by collapsed runs, then by the geometric mean over cells of their wire
bytes per byte of the stream read (`wire_per_useful`, the sims' `up_*`
ledger). Lag up to the delivery target is the room a trade may spend: a
cell whose median lag p99 the base kept under `--lag-ceiling` (analyze.py's
DELIVERY_TARGET_MS, delivery's share of the 2 s glass-to-glass target) is
out when the arm's isn't, and no ratio guards lag below it unless
`--lag-budget` sets one.

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
for strims-sim) at `off` and at each set, the tier's matrices in one
farm run (one boot of its instances, one tail). Each set is scored
against `off`, which stands for the base: with its flags off the commit
must reproduce the base's traces (`score` against a farm run of the base
shows whether it does). It prints each set's collapses and lag
p99 by cell, then its scorecard, against `off` or the set `--against`
names.
With `--refs` the arms are commits instead (the matrices' arms, no knob
axis), scored against the first. `--no-run` writes the matrices
(target/sim-suite/farm/RUN-NN.toml) and reports the rows the farm has
without launching anything: to quote them (`farm.py estimate`), or to run
several tiers' in one farm run (`farm.py run`) and report each after.

`score` scores arm B of one farm run against arm A of another (or the
same) run, paired by cell and seed: a new arm against base rows an earlier
run holds, without running the base again. RUN is the run's name as
`farm` prints it, ARM a `--refs` name or a `--sets` set (`off` for none).
`farm` keeps each run's index in the main checkout's
target/sim-suite/farm-runs/RUN.json once the farm has started the run, and
in farm-runs/pending/ until then (`--no-run`): the first `farm` or `score`
after the farm starts it moves it into place, and one whose run hasn't
started within a week is dropped. The index holds its cells as their
matrices ran them (name, sim, test, why, env and the slot's mem_gib), the
matrices' arms (name: commit), the seeds, the arm the others are scored
against (`base`), what the run asks (`--question`, for the sims'
catalog: hack/simlab/catalog.py) and for a `--sets` run its knob and
sets. A job's arm is named as hack/sim-db names it (simdb's
arm_label): in a run whose index has a knob, its set, the value its env
gives the knob; else its matrix's arm, a `--refs` name. So a `--sets`
run's arms are its sets alone, and a ref name never stands for one. A
run without an index is read as the cells of `--tier` and `--only` in the
order `farm` wrote them, which must be the run's, and its arms both ways,
its sets by `--knob`'s value: an ARM naming a set and a ref with other
rows is refused as ambiguous. A farm canary runs a ppspp cell's stream
for 2 s and a strims-sim cell's for 5 s, so it starts every viewer with
the seed and lets none leave when the cell's audience wouldn't fit in it
(an event, departures).

`list` prints a tier's cells: each one's sim, test, audience (arrivals
and departures), seeds and settings.

The tiers, cells and metrics, the local runs and the farm's matrices are
simlab.suite's (hack/simlab/suite.py), and the scorecards and arm scores
simlab.scoring's (hack/simlab/scoring.py): this script parses the
commands and prints what they give.
"""

import argparse
import hashlib
import itertools
import json
import math
import os
import shlex
import subprocess
import sys
import types
from pathlib import Path

HACK = Path(__file__).resolve().parents[1]
if str(HACK) not in sys.path:
    sys.path.insert(0, str(HACK))
from simlab import bootstrap, scoring, suite as lab, verdict  # noqa: E402


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


def ranked(a):
    """scoring.arm_score's ranking, as --rank gives it, with --lag-budget's
    and --lag-ceiling's when ranked for overhead."""
    if a.rank != "overhead":
        return {"rank": a.rank}
    return {"rank": a.rank, "lag_budget": a.lag_budget, "lag_ceiling": a.lag_ceiling}


def race(a, cells, count, sha, base, sims, union):
    flags = [f.strip() for f in a.flags.split(",") if f.strip()]
    knobs = [k.strip() for k in a.knob.split(",") if k.strip()]
    tree = {sim: lab.tree_binary(sim, base[sim]) for sim in sims}

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
        rows, failures = lab.run_cells(cells, runs, a.slots, a.mem_gib)
        base_rows = union(rows, [label for label, _ in base.values()])
        scores = {arm: scoring.arm_score(cells, base_rows, union(rows, labels[arm]), **ranked(a)) for arm in arms}
        print(f"\n{title}: {len(arms)} arms")
        print(f"   {'arm':58} collapsed (base)   p99 ratio  wire ratio   guardrails")
        for arm in sorted(arms, key=lambda x: scores[x].rank_key):
            sc = scores[arm]
            p99, wire, broken = scoring.ratio_text(sc.p99), scoring.ratio_text(sc.wire), sc.broken
            print(f"   {','.join(arm):58} {sc.collapsed:5} ({sc.base_collapsed:3})   {p99:>9}  {wire:>10}   "
                  f"{'; '.join(broken[:3]) + (' ...' if len(broken) > 3 else '') or 'ok'}")
            for c in sc.cells if per_cell else ():
                p99 = "-" if c["lag"] is None else f"x{c['lag']:.2f} [{c['lag_lo']:.2f},{c['lag_hi']:.2f}]"
                print(f"      {c['cell']:22} collapsed {c['ka']:>3} -> {c['kb']:>3} of {c['n']:<3}  lag p99 {p99}")
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
        # The base itself, all off.
        base_arm = types.SimpleNamespace(collapsed=scores[arms[0]].base_collapsed, p99=1.0, breaks=[])
        off = [scores[x] for x in arms if f not in x] + [base_arm]
        lp = lambda xs: sum(math.log(x.p99) for x in xs if x.p99) / max(1, sum(1 for x in xs if x.p99))
        col = lambda xs: sum(x.collapsed for x in xs) / max(1, len(xs))
        out = lambda xs: sum(1 for x in xs if x.breaks) / max(1, len(xs))
        print(f"   {f:18} {lp(on) - lp(off):+.3f}   {col(on) - col(off):+6.2f}   {out(on):.0%} vs {out(off):.0%}")
    for mult in (1.0, 2.0):
        live = sorted(arms, key=lambda x: scores[x].rank_key)
        live = [x for x in live if not scores[x].breaks] or live[:1]
        arms = live[: max(a.keep, math.ceil(len(live) / 2))]
        scores = play(arms, upto(mult), f"round at {mult:g}x the seeds")
    best = sorted(arms, key=lambda x: scores[x].rank_key)[: a.keep]
    fresh = lambda c: list(range(1001, 1001 + max(2, count(c))))
    play(best, fresh, "confirmation at fresh seeds", per_cell=True)


def farm(a, cells):
    arms = {n.strip(): r.strip() for n, r in (x.split("=", 1) for x in a.refs.split(","))} if a.refs else None
    m = lab.write_matrices(
        cells, range(a.first_seed, a.first_seed + a.seeds), arms=arms,
        sets=None if a.refs else [x.strip() for x in a.sets.split(";") if x.strip()], ref=a.ref,
        knob=dict(zip((lab.PPSPP, lab.SIM), (k.strip() for k in a.knob.split(",")))), env=a.env,
        question=a.question, against=a.against.strip() if a.against else None, no_off=a.no_off)
    name, index = m.name, m.index
    arms, base = index["arms"], index["base"]
    values = list(arms) if a.refs else index["values"]
    sha = next(iter(arms.values()))
    if not a.no_run:
        lab.place_index(name, index, True)
        print(f"farm run {name}: {len(cells)} matrices, {len(values)} arms x {len(index['seeds'])} seeds",
              file=sys.stderr)
        # One run for the tier: one boot of its instances, and one tail.
        farm_py = lab.farm_root() / "hack" / "sim-farm" / "farm.py"
        subprocess.run([sys.executable, str(farm_py), "run", *(x.path for x in m.matrices),
                        *shlex.split(a.farm_args)])
        if not lab.farm_started(name, len(cells)):
            lab.place_index(name, index, False)
    rows = lab.farm_rows(name, cells, index.get("knob"))
    others = [v for v in values if v != base]
    # A --refs arm is its own commit; a --sets arm runs the one commit.
    label = (lambda v: f"{arms[v][:8]} {v}") if a.refs else (lambda v: f"{sha[:8]} {v}")
    for v in others:
        title = f"{label(v)} against {label(base)}" if a.refs else f"{v} against {base} at {sha[:8]}"
        print(scoring.arm_score(cells, rows[base], rows[v], title, **ranked(a)).text(), end="")
    for v in others:
        print()
        print(scoring.scorecard(cells, rows[base], rows[v], label(base), label(v), a.all).text(), end="")


def list_cells(a):
    """`list`: each cell of the tier (every tier's without one), with its
    audience and settings, the base envs' common keys left out."""
    common = {**lab.PP_BASE, **lab.SS_BASE}
    for tier in [a.tier] if a.tier else lab.TIERS:
        cells = lab.cells(tier, a.only.split(",") if a.only else None)
        print(f"{tier}: {len(cells)} cells")
        for c in cells:
            env = dict(c["env"])
            arrivals = env.pop(lab.ARRIVALS[c["sim"]], "t0")
            depart = env.pop(lab.DEPART[c["sim"]], "")
            rest = " ".join(f"{short(k)}={v}" for k, v in sorted(env.items()) if common.get(k) != v)
            print(f"   {c['name']:32} {c['sim']:10} {c['test']:46} {arrivals:15} {depart or '-':16} "
                  f"seeds {c['seeds'] or '-':<3} {rest}")


def short(k):
    """A knob's name without its sim's prefix."""
    for p in ("STRIMS_SIM_CAL_", "STRIMS_SIM_", "SIM_"):
        if k.startswith(p):
            return k[len(p):]
    return k


def score(a, cells):
    knob = dict(zip((lab.PPSPP, lab.SIM), (k.strip() for k in a.knob.split(",")))) if a.knob else None
    lab.settle_indexes()
    picked = []
    for arg in (a.a, a.b):
        run, sep, arm = arg.rpartition(":")
        if not sep or not run:
            sys.exit(f"{arg!r} isn't RUN:ARM")
        index = lab.farm_index(run)
        index = json.loads(index.read_text()) if index.exists() else None
        run_cells = index["cells"] if index else cells
        picked.append((run_cells, lab.arm_rows(run, run_cells, arm, index, knob)))
    (a_cells, a_rows), (b_cells, b_rows) = picked
    names = {c["name"] for c in a_cells}
    both = [c for c in b_cells if c["name"] in names]
    if a.only:
        both = [c for c in both if c["name"] in set(a.only.split(","))]
    if a.rank == "overhead":
        print(scoring.arm_score(both, a_rows, b_rows, f"{a.b} against {a.a}", **ranked(a)).text(), end="")
        print()
    print(scoring.scorecard(both, a_rows, b_rows, a.a, a.b, a.all).text(), end="")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)
    cal = sub.add_parser("calibrate")
    cal.add_argument("--base", default="main")
    cal.add_argument("--tier", default="peers", choices=sorted(lab.TIERS))
    cal.add_argument("--seeds", type=int, default=4)
    cal.add_argument("--slots", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    cal.add_argument("--mem-gib", type=float, default=40)
    cal.add_argument("--only", help="cells, comma-separated")
    cal.add_argument("--rebuild", action="store_true")
    r = sub.add_parser("run")
    r.add_argument("--base", default="main")
    r.add_argument("--tier", default="peers", choices=sorted(lab.TIERS))
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
    rc.add_argument("--knob", default=lab.DEFAULT_KNOB,
                    help="the environment knobs that take the flags, comma-separated")
    rc.add_argument("--base", default="main")
    rc.add_argument("--tier", default="peers", choices=sorted(lab.TIERS))
    rc.add_argument("--seeds", type=int, default=4)
    rc.add_argument("--slots", type=int, default=max(1, (os.cpu_count() or 2) - 1))
    rc.add_argument("--mem-gib", type=float, default=40)
    rc.add_argument("--only", help="cells, comma-separated")
    rc.add_argument("--rebuild", action="store_true")
    rc.add_argument("--keep", type=int, default=3, help="arms the confirmation keeps")
    fc = sub.add_parser("farm")
    fc.add_argument("--sets", help="flag sets, ;-separated, each comma-separated")
    fc.add_argument("--refs", help="commits instead, name=ref,...: the matrices' arms, the first the base")
    fc.add_argument("--knob", default=lab.DEFAULT_KNOB,
                    help="the knobs that take a set: the ppspp sim's, then strims-sim's")
    fc.add_argument("--ref", default="HEAD", help="the commit the farm builds")
    fc.add_argument("--tier", default="peers", choices=sorted(lab.TIERS))
    fc.add_argument("--seeds", type=int, default=32, help="seeds per cell, every cell alike")
    fc.add_argument("--first-seed", type=int, default=2001)
    fc.add_argument("--against", help="the set or arm the others are scored against (off, or the first ref)")
    fc.add_argument("--question", help="what the run asks, kept in its index (the sims' catalog's experiments)")
    fc.add_argument("--only", help="cells, comma-separated")
    fc.add_argument("--all", action="store_true", help="every metric, ties too")
    fc.add_argument("--farm-args", default="", help="more arguments for farm.py run")
    fc.add_argument("--no-run", action="store_true",
                    help="write the matrices and report the rows the farm has, launching nothing")
    fc.add_argument("--no-off", action="store_true", help="run only the sets given, not `off` beside them")
    sc = sub.add_parser("score")
    sc.add_argument("a", metavar="RUN:ARM", help="the base: a farm run's name and one of its arms")
    sc.add_argument("b", metavar="RUN:ARM", help="the arm scored against it")
    sc.add_argument("--tier", default="peers", choices=sorted(lab.TIERS),
                    help="a run's cells when it has no index: the tier it ran")
    sc.add_argument("--only", help="cells, comma-separated (the run's, when it has no index)")
    sc.add_argument("--knob", help="the knobs that took the sets of a --sets run without an index: the ppspp "
                                   f"sim's, then strims-sim's (default {lab.DEFAULT_KNOB}; an index names its own)")
    sc.add_argument("--all", action="store_true", help="every metric, ties too")
    ls = sub.add_parser("list")
    ls.add_argument("--tier", choices=sorted(lab.TIERS), help="one tier (default: every one)")
    ls.add_argument("--only", help="cells, comma-separated")
    for p in (cal, r, rc, fc):
        p.add_argument("--env", action="append", default=[], metavar="KEY=VALUE",
                       help="set for every arm's runs, as part of each cell")
    for p in (rc, fc, sc):
        p.add_argument("--rank", choices=("lag", "overhead"), default="lag",
                       help="rank arms after collapses by lag p99, or by wire bytes per useful byte")
        p.add_argument("--lag-budget", type=float, default=None,
                       help="with --rank overhead: a ratio guarding lag below the ceiling (none by default)")
        p.add_argument("--lag-ceiling", type=float, default=verdict.DELIVERY_TARGET_MS,
                       help="with --rank overhead: the lag p99 (ms) a cell under it must stay under "
                            "(delivery's share of the latency target, analyze.py)")
    a = ap.parse_args()

    if a.cmd == "list":
        list_cells(a)
        return
    only = a.only.split(",") if a.only else None
    if a.cmd == "score":
        score(a, lab.cells(a.tier, only))
        return
    cells = lab.cells(a.tier, only, a.env)
    if a.cmd == "farm":
        farm(a, cells)
        return
    count = lambda c: c["seeds"] or a.seeds
    seeds = lambda c: list(range(1, count(c) + 1))
    sha = lab.git("rev-parse", a.base)
    sims = sorted({c["sim"] for c in cells})
    base = {sim: lab.base_binary(sha, sim, a.rebuild) for sim in sims}
    union = lambda rows, labels: {k: r for label in labels for k, r in rows[label].items()}
    if a.cmd == "race":
        race(a, cells, count, sha, base, sims, union)
        return
    if a.cmd == "calibrate":
        both = lambda c: list(range(1, 2 * count(c) + 1))
        runs = {label: (bin_dir, both, sim, {}) for sim, (label, bin_dir) in base.items()}
        rows, failures = lab.run_cells(cells, runs, a.slots, a.mem_gib)
        every = union(rows, runs)
        per = {c["name"]: count(c) for c in cells}
        b = {(c, s - per[c]): r for (c, s), r in every.items() if s > per[c]}
        a_rows = {k: r for k, r in every.items() if k[1] <= per[k[0]]}
        report = lambda: scoring.scorecard(cells, a_rows, b, f"{a.base} {sha[:8]} at seeds 1..N",
                                           "the same at seeds N+1..2N", True, traces=False)
    else:
        tree = {sim: lab.tree_binary(sim, base[sim]) for sim in sims}
        # The working tree's knobs name its rows apart from the same binary's
        # without them.
        env = dict(kv.split("=", 1) for kv in a.tree_env)
        if env:
            tag = hashlib.sha1(json.dumps(sorted(env.items())).encode()).hexdigest()[:8]
            tree = {sim: (f"{label}~{tag}", bin_dir) for sim, (label, bin_dir) in tree.items()}
        runs = {label: (bin_dir, seeds, sim, {}) for sim, (label, bin_dir) in base.items()}
        runs.update({label: (bin_dir, seeds, sim, env) for sim, (label, bin_dir) in tree.items()})
        rows, failures = lab.run_cells(cells, runs, a.slots, a.mem_gib)
        a_rows = union(rows, [label for label, _ in base.values()])
        b_rows = union(rows, [label for label, _ in tree.values()])
        tree_labels = ", ".join(label for label, _ in tree.values())
        knobs = " ".join(a.tree_env)
        report = lambda: scoring.scorecard(cells, a_rows, b_rows, f"{a.base} {sha[:8]}",
                                           f"working tree ({tree_labels}){' with ' + knobs if knobs else ''}",
                                           a.all)
    if failures:
        print("FAILURES")
        for f in failures:
            print("  " + f)
    print(report().text(), end="")
    if failures:
        sys.exit(1)


def __getattr__(name):
    """simlab.suite's names (the tiers, metrics, runs and farm matrices this
    script's commands call), else simlab.scoring's (its scorecards and arm
    scores), for the scripts that read them here."""
    if hasattr(lab, name):
        return getattr(lab, name)
    return getattr(scoring, name)


if __name__ == "__main__":
    bootstrap.engine(__file__)
    main()
