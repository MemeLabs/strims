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
                               [--first-seed N] [--against SET]
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
`farm` keeps each run's cells in the main checkout's
target/sim-suite/farm-runs/RUN.json once the farm has started the run, and
in farm-runs/pending/ until then (`--no-run`): the first `farm` or `score`
after the farm starts it moves it into place, and one whose run hasn't
started within a week is dropped. A run without one is read as the cells
of `--tier` and `--only` in the order `farm` wrote them, which must be the
run's. A farm canary runs a ppspp cell's stream for 2 s and a strims-sim
cell's for 5 s, so it starts every viewer with the seed and lets none
leave when the cell's audience wouldn't fit in it (an event, departures).

`list` prints a tier's cells: each one's sim, test, audience (arrivals
and departures), seeds and settings.
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
import re
import shlex
import shutil
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "target" / "sim-suite"
CARGO = WORK / "cargo"
# Cargo takes a unit as fresh while its sources are older than its last
# build, and a base's checkout rewrites only the files that differ from the
# last base's: had the working tree built in between, a base would take its
# units for every file it shares with the last base. The tree builds apart.
TREE_CARGO = WORK / "cargo-tree"
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

# Each sim's audience knobs (see the module docs): when the viewers arrive,
# and who leaves for good.
ARRIVALS = {PPSPP: "SIM_ARRIVALS", SIM: "STRIMS_SIM_ARRIVALS"}
DEPART = {PPSPP: "SIM_DEPART", SIM: "STRIMS_SIM_DEPART"}
# The cells whose viewers trickle in (the audience default), by sim and
# name, and how: every scripted event of theirs comes after the last
# arrival and its grace. The rest keep every viewer at the start: the
# collapse reproducers (lock-in, star-31-5, the halvings, thin, the slow
# viewer), the fixed-fault cells (seed-starved, latency-step, seed-restart,
# viewer-churn, capped-seed, the capped hybrid) and churn. The 30 s ppspp
# cells stream for 45 s (TRICKLE_SECS) so their last arrivals count for
# 20 s; the uplink step's arrivals come at a second each, done well before
# it, as its stream's first third would not be.
TRICKLE = {
    **{(PPSPP, n): "trickle" for n in ("large-2mbit", "large-4mbit", "overload", "grabbers", "no-greedy",
                                        "wan-global", "wan-mobile", "sparse-lossy", "residential")},
    **{(SIM, n): "trickle" for n in ("star-15", "residential-uncapped", "residential-varying", "two-streams",
                                      "mobile-uncapped")},
    **{(SIM, f"star-{v}-{m}"): "trickle" for v in (3, 7, 15, 31) for m in (5, 10, 20) if (v, m) != (31, 5)},
    **{(SIM, n): "trickle" for n in ("mesh6-31-10", "mesh6-15-5", "jitter-15-10", "limited-15-10",
                                      "wan-residential-seed-adaptive", "wan-super-peer-adaptive",
                                      "wan-residential-seed", "wan-super-peer", "wan-global-audience",
                                      "wan-mobile-viewers", "wan-residential-two-streams",
                                      "wan-residential-seed-varying", "wan-super-peer-varying")},
    (SIM, "wan-residential-seed-step"): "trickle:1",
}
TRICKLE_SECS = {"30": "45"}


def audience(c, arrivals=None, depart=None, **env):
    """Cell `c` with its viewers arriving by `arrivals` (TRICKLE's by
    default; "t0" drops the knob) and `depart` leaving."""
    env = {**c["env"], **env}
    arrivals = arrivals or TRICKLE.get((c["sim"], c["name"]))
    if arrivals == "t0":
        env.pop(ARRIVALS[c["sim"]], None)
    elif arrivals:
        env[ARRIVALS[c["sim"]]] = arrivals
        if c["sim"] == PPSPP and env.get("SIM_SECS") in TRICKLE_SECS:
            env["SIM_SECS"] = TRICKLE_SECS[env["SIM_SECS"]]
    if depart:
        env[DEPART[c["sim"]]] = depart
    return {**c, "env": env}


# The audience cells' profiles (see the module docs): a scheduled event's
# burst (a fifth of the viewers trickling in over its first 20 s, the rest
# within 10 s from there), its end (60 % of those there by 40 s leaving
# within 10 s, gracefully or abruptly), and both, the end 30 s after the
# burst, each with the stream it needs to run on past them.
EVENT = "event:20:20:10"
END = "60:40:10"
EVENT_END = "60:60:10"


def audience_cells(c, secs_key, name=None, why=None):
    """The audience cells of `c` (its name, or `name`): the event, the end
    gracefully and abruptly, and both, at the tier's seeds. `secs_key` is
    the knob for its stream's length."""
    name = name or c["name"]
    why = why or c["why"]
    out = []
    for suffix, arrivals, depart, secs, what in (
            ("event", EVENT, None, "60", "a scheduled event's burst into a warm fifth"),
            ("end", "trickle", END, "90", "60 % leaving within 10 s from 40 s in"),
            ("end-abrupt", "trickle", f"{END}:abrupt", "90", "60 % going silent within 10 s from 40 s in"),
            ("combined", EVENT, EVENT_END, "90", "an event's burst at 20 s, then 60 % leaving within 10 s from 60 s")):
        x = audience(c, arrivals, depart, **{secs_key: secs})
        out.append({**x, "name": f"{name}-{suffix}", "why": f"{why}: {what}", "seeds": None})
    return out


def robust_tier():
    """The real stack across shapes, for a change that must hold in
    general: stars of 3 to 31 viewers behind 5 to 20 Mbit/s seeds, steady
    and with the seed's uplink halved, sparse meshes, jitter, rate limits,
    small relay uplinks, and every WAN, step, starvation, churn and restart
    case of strims-sim's Hybrid streams, their viewers trickling in where
    TRICKLE says; then the audience cells (`audience_cells`) of a star of
    15 behind a 10 Mbit/s seed (a full mesh, which no departure can
    strand), a residential seed's event and the thin relays' shape
    trickling in (thin-15-10 itself reproduces a collapse)."""
    cells = robust_t0()
    by = {c["name"]: c for c in cells}
    star = by["star-15-10"]
    thin = audience(by["thin-15-10"], "trickle")
    out = [audience(c) for c in cells]
    out.append({**thin, "name": "thin-15-10-trickle", "why": f"{thin['why']}, trickling in"})
    out += audience_cells(star, "STRIMS_SIM_CAL_SECS")
    wan = by["wan-residential-seed"]
    out.append({**audience(wan, EVENT), "name": "wan-residential-seed-event",
                "why": f"{wan['why']}: a scheduled event's burst into a warm fifth"})
    return out


def robust_t0():
    """robust_tier's cells as they are with every viewer at the start."""
    star = lambda v, m, **env: {**SS_STAR, "STRIMS_SIM_CAL_VIEWERS": str(v), "STRIMS_SIM_CAL_SEED_UP_MBIT": str(m),
                                "STRIMS_SIM_CAL_SECS": "60", **env}
    cells = [cell(f"star-{v}-{m}", SIM, "calibration_star", star(v, m), 1, f"{v} viewers, a {m} Mbit/s seed")
             for v in (3, 7, 15, 31) for m in (5, 10, 20)]
    cells += [cell(f"halve-{v}-{m}", SIM, "calibration_star",
                   star(v, m, STRIMS_SIM_CAL_SECS="70", STRIMS_SIM_CAL_SEED_UP_SCALE="30:0.5"), 1,
                   f"{v} viewers, a {m} Mbit/s seed halved at 30 s")
              for v in (7, 15, 31) for m in (10, 20)]
    cells += [
        cell("mesh6-31-10", SIM, "calibration_star", star(31, 10, STRIMS_SIM_CAL_MESH="mesh6"), 1,
             "31 viewers in a 6-link mesh, a 10 Mbit/s seed"),
        cell("mesh6-15-5", SIM, "calibration_star", star(15, 5, STRIMS_SIM_CAL_MESH="mesh6"), 1,
             "15 viewers in a 6-link mesh, a 5 Mbit/s seed"),
        cell("jitter-15-10", SIM, "calibration_star", star(15, 10, STRIMS_SIM_CAL_JITTER="1"), 1,
             "15 viewers on jittery cable, a 10 Mbit/s seed"),
        cell("limited-15-10", SIM, "calibration_star", star(15, 10, STRIMS_SIM_CAL_RATE_LIMIT_PCT="80"), 1,
             "15 viewers, every node rate limited to 80 % of its uplink"),
        cell("thin-15-10", SIM, "calibration_star", star(15, 10, STRIMS_SIM_CAL_VIEWER_UP_MBIT="3"), 1,
             "15 viewers with 3 Mbit/s uplinks, a 10 Mbit/s seed"),
    ]
    cells += [cell(t.replace("_adaptive_uncapped", "").replace("_hybrid", "").replace("_", "-"), SIM, t, SS_BASE, 1, t)
              for t in ("wan_residential_seed_adaptive", "wan_super_peer_adaptive",
                        "wan_residential_seed_adaptive_uncapped", "wan_super_peer_adaptive_uncapped",
                        "wan_global_audience_adaptive_uncapped", "wan_mobile_viewers_adaptive_uncapped",
                        "wan_residential_two_streams_adaptive_uncapped", "wan_slow_viewer_adaptive_uncapped",
                        "wan_residential_seed_adaptive_uncapped_varying",
                        "wan_super_peer_adaptive_uncapped_varying", "wan_residential_seed_adaptive_uncapped_step",
                        "latency_step_hybrid", "capped_seed_hybrid", "seed_starved_hybrid", "viewer_churn_hybrid",
                        "seed_restart_hybrid")]
    return cells


# A browser audience for STRIMS_SIM_RTC: three hosts in four on Chromium's
# SCTP (Chrome, Edge and Safari run libwebrtc's dcSCTP), one on Firefox's
# usrsctp, so swarms have links between the stacks.
BROWSERS = "chromium,chromium,chromium,firefox"


def browsers_tier():
    """Robust's cells that run the window model (the stars and the uncapped
    WAN cases; the others run no data channel model), every host a browser
    (BROWSERS)."""
    return [{**c, "env": {**c["env"], "STRIMS_SIM_RTC": BROWSERS}, "why": f"{c['why']}, as browsers"}
            for c in TIERS["robust"] if c["test"] == "calibration_star" or "uncapped" in c["test"]]


# The net tier's cells, robust's names: as browsers, and these over TCP.
NET_RTC = ["star-7-5", "star-15-10", "star-15-20", "star-31-10", "star-31-20", "halve-15-10", "halve-31-20",
           "jitter-15-10", "thin-15-10", "wan-residential-seed", "wan-global-audience", "wan-mobile-viewers",
           "wan-residential-two-streams", "wan-slow-viewer", "wan-super-peer-varying", "wan-residential-seed-step",
           "star-15-10-event", "star-15-10-end", "star-15-10-end-abrupt", "star-15-10-combined"]
NET_TCP = ["star-15-10", "star-31-10", "thin-15-10", "wan-global-audience", "wan-residential-two-streams"]


def net_tier():
    """Organic networks (STRIMS_SIM_DISCOVERY): every host keeps only its
    link to the signalling relay and finds the others through transfer
    discovery, then links by peer exchange; robust's window-model stars and
    uncapped WAN cases across the conditions, as browsers (BROWSERS) and,
    some, over TCP; and a residential seed's event ending, gracefully and
    abruptly, where discovery replaces the neighbours that left (a static
    mesh would strand the viewers whose every link left)."""
    robust = {c["name"]: c for c in TIERS["robust"]}
    org = {"STRIMS_SIM_DISCOVERY": "1"}
    cells = [{**robust[n], "name": f"org-{n}", "env": {**robust[n]["env"], **org, "STRIMS_SIM_RTC": BROWSERS},
              "why": f"{robust[n]['why']}, as browsers, organic"} for n in NET_RTC]
    cells += [{**robust[n], "name": f"org-{n}-tcp", "env": {**robust[n]["env"], **org},
               "why": f"{robust[n]['why']}, organic"} for n in NET_TCP]
    wan = robust["wan-residential-seed"]
    ends = audience_cells(wan, "STRIMS_SIM_STREAM_SECS", "org-wan-residential-seed",
                          f"{wan['why']}, as browsers, organic")
    cells += [{**c, "env": {**c["env"], **org, "STRIMS_SIM_RTC": BROWSERS}} for c in ends
              if c["name"].endswith(("-end", "-end-abrupt"))]
    return cells


# The net-hostile tier's cells (net's names) and hostile kinds.
NET_HOSTILE = ["org-star-15-20", "org-star-31-10", "org-thin-15-10", "org-wan-residential-seed",
               "org-wan-global-audience"]


def net_hostile_tier():
    """The net tier's browser cells with a fifth of the viewers hostile
    (STRIMS_SIM_HOSTILE): free riders, then liars. The row splits the
    honest viewers' lag and delivery from the hostile ones'."""
    net = {c["name"]: c for c in TIERS["net"]}
    return [{**net[n], "name": f"{n}-{k}", "env": {**net[n]["env"], "STRIMS_SIM_HOSTILE": f"{k}:0.2"},
             "why": f"{net[n]['why']}, a fifth {k}s"} for k in ("free-rider", "liar") for n in NET_HOSTILE]


# The streams the sign tiers run# The streams the sign tiers run (strims_wan::live's encoders): audio alone
# at 128 kbit/s, then audio and video, CBR and VBR, to 6 Mbit/s.
SIGN_MEDIA = ["128c", "512c", "1500c", "1500v", "3000c", "6000c", "6000v"]
SIGN_CONFIRM_MEDIA = ["128c", "512c", "1500v", "3000c", "6000c"]
SIGN_CONFIRM_SIM_MEDIA = ["128c", "1500v", "6000c"]
# The sign tier's VBR streams at other keyframe ratios and scene spreads.
SIGN_SENS_MEDIA = ["1500v:k3", "1500v:k12", "1500v:s0.35", "1500v:s1.2", "6000v:k12", "6000v:s1.2"]


def media_cells(cells, media, knob):
    """Each of `cells` at each stream of `media`, set by `knob`."""
    return [cell(f"{c['name']}@{m}", c["sim"], c["test"], {**c["env"], knob: m}, c["mem_gib"],
                 f"{c['why']}, streaming {m}") for c in cells for m in media]


SIGN_PPSPP = ["wan-global", "wan-mobile", "residential", "sparse-lossy", "overload", "churn"]
SIGN_SIM = ["star-15-20", "star-15-10", "halve-15-20", "mesh6-31-10", "wan-residential-seed",
            "wan-residential-seed-varying", "wan-super-peer-varying", "wan-mobile-viewers", "seed-starved",
            "viewer-churn"]


def sign_tier():
    """How a live stream signs early, for a screen of sign policies (the
    knob SIM_SIGN): the peers tier's ppspp cells on real-world uplinks
    (wan-global, wan-mobile, residential; cable seeds of 100 viewers, steady
    and churning), sparse-lossy's mesh, and 200 viewers at the higher rates,
    each at each stream of SIGN_MEDIA."""
    peers = {c["name"]: c for c in TIERS["peers"]}
    cells = media_cells([peers[n] for n in SIGN_PPSPP], SIGN_MEDIA, "SIM_MEDIA")
    return cells + media_cells([peers["large-4mbit"]], ["3000c", "6000c", "6000v"], "SIM_MEDIA")


def sign_confirm_tier():
    """The sign tier's ppspp cells at SIGN_CONFIRM_MEDIA's streams, and the
    real stack's (strims-sim, the knob STRIMS_SIM_SIGN) at
    SIGN_CONFIRM_SIM_MEDIA's: stars behind 10 and 20 Mbit/s seeds, a
    halving, a sparse mesh, residential and super-peer seeds on steady and
    varying uplinks, mobile viewers, a starved seed and viewers that crash."""
    peers = {c["name"]: c for c in TIERS["peers"]}
    robust = {c["name"]: c for c in TIERS["robust"]}
    return (media_cells([peers[n] for n in SIGN_PPSPP], SIGN_CONFIRM_MEDIA, "SIM_MEDIA")
            + media_cells([robust[n] for n in SIGN_SIM], SIGN_CONFIRM_SIM_MEDIA, "STRIMS_SIM_MEDIA"))


def sign_sens_tier():
    """The sign tier's ppspp cells at SIGN_SENS_MEDIA's streams: how much a
    policy's standing turns on the encoder model."""
    peers = {c["name"]: c for c in TIERS["peers"]}
    return media_cells([peers[n] for n in SIGN_PPSPP], SIGN_SENS_MEDIA, "SIM_MEDIA")


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
        cell("residential-varying", SIM, "wan_residential_seed_adaptive_uncapped_varying", SS_BASE, 1,
             "the same seed's uplink varying: every viewer gets every frame", CHEAP),
        cell("two-streams", SIM, "wan_residential_two_streams_adaptive_uncapped", SS_BASE, 1,
             "two streams from one residential seed share every uplink", CHEAP),
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
    "robust": robust_t0(),
}


def peers_audience(peers):
    """The peers tier's audience cells (`audience_cells`): the churn cell's
    swarm (a cable seed's 100 viewers at 4 Mbit/s) and the real stack's
    healthy star (`peers`: the tier's cells)."""
    cable = cell("cable", PPSPP, "sim_job", {**PP_CABLE, "SIM_PEERS": "100", "SIM_MBIT": "4"}, 3,
                 "100 viewers of a cable seed at 4 Mbit/s")
    star = next(c for c in peers if c["name"] == "star-15")
    return audience_cells(cable, "SIM_SECS") + audience_cells(star, "STRIMS_SIM_CAL_SECS")


# Today's cells that the audience default moves, as an explicit stress test
# under their names, then the default and the audience cells.
TIERS["startup"] = [c for t in ("peers", "robust") for c in TIERS[t] if (c["sim"], c["name"]) in TRICKLE]
TIERS["peers"] = [audience(c) for c in TIERS["peers"]] + peers_audience(TIERS["peers"])
TIERS["robust"] = robust_tier()


def recip_tier():
    """Organic browsers whose uplinks differ, for where strong ones sit
    (reciprocal topology): the super-peer cells, a residential crowd of 31
    and of 63, and the stars and WAN cells the pool touches most; the
    crowds trickle in, as organic cells do."""
    net = {c["name"]: c for c in TIERS["net"]}
    cells = [net[n] for n in ("org-wan-super-peer-varying", "org-star-31-10", "org-thin-15-10",
                              "org-wan-global-audience")]
    org = {"STRIMS_SIM_DISCOVERY": "1", "STRIMS_SIM_RTC": BROWSERS, "STRIMS_SIM_ARRIVALS": "trickle"}
    for test, name, crowd in (("wan_residential_crowd_adaptive_uncapped", "org-crowd-31", "31"),
                              ("wan_residential_crowd_adaptive_uncapped", "org-crowd-63", "63"),
                              ("wan_super_peer_crowd_adaptive_uncapped_varying", "org-super-peer-crowd-31", "31")):
        cells.append(cell(name, SIM, test, {**SS_BASE, **org, "STRIMS_SIM_CROWD": crowd}, 2,
                          f"{crowd} residential viewers, as browsers, organic"))
    return cells


def recip_hostile_tier():
    """The residential crowd of 31 with a fifth of its viewers each kind of
    defector, and with viewers leaving and rejoining (churn: every viewer
    there from the start, as churn's cells keep them)."""
    crowd = next(c for c in TIERS["recip"] if c["name"] == "org-crowd-31")
    cells = [{**crowd, "name": f"org-crowd-31-{k}", "env": {**crowd["env"], "STRIMS_SIM_HOSTILE": f"{k}:0.2"},
              "why": f"{crowd['why']}, a fifth {k}s"} for k in ("free-rider", "liar", "whitewasher", "hog")]
    churn = audience(crowd, "t0", STRIMS_SIM_CHURN="0.5:5")
    cells.append({**churn, "name": "org-crowd-31-churn",
                  "why": f"{crowd['why']}, a viewer leaving for 5 s every 2 minutes"})
    return cells


TIERS["browsers"] = browsers_tier()
TIERS["net"] = net_tier()
TIERS["net-hostile"] = net_hostile_tier()
TIERS["recip"] = recip_tier()
TIERS["recip-hostile"] = recip_hostile_tier()
TIERS["sign"] = sign_tier()
TIERS["sign-confirm"] = sign_confirm_tier()
TIERS["sign-sens"] = sign_sens_tier()

# Metrics: (name, how to read it from a row, better: "lower", "higher" or
# None, kind: "ratio" (geometric) or "diff" (absolute), tie band).
LOWER, HIGHER = "lower", "higher"


def depth_mean(r):
    """The mean hops to the seed over chains that reach it (ppspp sim)."""
    hops = [(k, r.get(f"depth_{k}")) for k in range(1, 7)]
    w = sum(v for _, v in hops if v)
    return sum(k * v for k, v in hops if v) / w if w else None


def derived(r):
    """A row's verdict and run metrics (analyze.py's), from its tables'
    series: simdb's rows, local or the farm's, carry them (`_verdict`,
    `_metrics`)."""
    return r.get("_verdict"), r.get("_metrics")


def unhealthy(r):
    v = derived(r)[0]
    return v["unhealthy"] if v else None


def metric(name):
    """A run metric (analyze.run_metrics) of a row."""
    return lambda r: (derived(r)[1] or {}).get(name)


def share(num, den):
    return lambda r: (r.get(num, 0) or 0) / den(r) if den(r) else None


def per_useful(*fields):
    """The row's `fields` summed, per byte of the stream the honest viewers
    read (`useful_bytes`)."""
    def get(r):
        if not r.get("useful_bytes") or any(f not in r for f in fields):
            return None
        return sum(r[f] for f in fields) / r["useful_bytes"]
    return get


HOST_METRICS = ("rss_mb", "wall_s")

METRICS = [
    # A row with hostile viewers scores its honest ones'.
    ("lag_p50_ms", lambda r: r.get("lag_p50_ms_honest", r.get("lag_p50_ms")), LOWER, "ratio", 0.05),
    ("lag_p99_ms", lambda r: r.get("lag_p99_ms_honest", r.get("lag_p99_ms")), LOWER, "ratio", 0.05),
    ("unhealthy", unhealthy, LOWER, "diff", 0.03),
    # The run's health over time (analyze.run_metrics).
    ("collapses", metric("collapses"), LOWER, "diff", 0.5),
    ("flaps_per_min", metric("flaps_per_min"), LOWER, "diff", 0.2),
    ("unhealthy_dwell_p90_s", metric("unhealthy_dwell_p90_s"), LOWER, "diff", 2),
    ("onset_after_drop_s", metric("onset_after_drop_s"), HIGHER, "diff", 2),
    ("recover_after_rise_s", metric("recover_after_rise_s"), LOWER, "diff", 2),
    ("excess_lag_area", metric("excess_lag_area_ms_s"), LOWER, "ratio", 0.10),
    ("osc_amp_ms", metric("osc_amp_ms"), LOWER, "ratio", 0.10),
    ("delivery", lambda r: r.get("delivery_honest", r.get("delivery")), HIGHER, "diff", 0.002),
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
    # Each viewer from its own join (with arrivals or departures): to its
    # first read or frame, to its first under the delivery target, the
    # viewers that never did.
    ("join_p50_ms", lambda r: r.get("join_p50_ms"), LOWER, "ratio", 0.10),
    ("join_p90_ms", lambda r: r.get("join_p90_ms"), LOWER, "ratio", 0.10),
    ("caught_up_p50_ms", lambda r: r.get("caught_up_p50_ms"), LOWER, "ratio", 0.10),
    ("caught_up_p90_ms", lambda r: r.get("caught_up_p90_ms"), LOWER, "ratio", 0.10),
    ("caught_up_never", lambda r: r.get("caught_up_never"), LOWER, "diff", 0.5),
    # The viewers that stayed through departures: their worst lag, their lag
    # p90's excess over the delivery target summed, the time back under it,
    # those left with no neighbour; and how soon a child noticed its parent
    # gone (strims-sim).
    ("depart_lag_peak_ms", lambda r: r.get("depart_lag_peak_ms"), LOWER, "ratio", 0.10),
    ("depart_excess_area", lambda r: r.get("depart_excess_lag_area_ms_s"), LOWER, "ratio", 0.10),
    ("depart_recover_ms", lambda r: r.get("depart_recover_ms"), LOWER, "diff", 2000),
    ("depart_stranded", lambda r: r.get("depart_stranded"), LOWER, "diff", 0.5),
    ("parent_lost_noticed_p90_ms", lambda r: r.get("parent_lost_noticed_p90_ms"), LOWER, "ratio", 0.10),
    ("startup_p99_ms", lambda r: r.get("startup_p99_ms_honest", r.get("startup_p99_ms")), LOWER, "ratio", 0.10),
    ("signed_lag_p99_ms", lambda r: r.get("signed_lag_p99_ms"), LOWER, "ratio", 0.05),
    ("wire/media", lambda r: r.get("wire_per_media"), LOWER, "ratio", 0.02),
    # Where the uplinks' bytes go (the sims' `up_*` ledger), per byte read.
    ("wire/useful", lambda r: r.get("wire_per_useful"), LOWER, "ratio", 0.02),
    ("packets/useful", per_useful("packets_up"), LOWER, "ratio", 0.02),
    ("dup/useful", per_useful("dup_bytes"), LOWER, "diff", 0.002),
    ("ovh_seed", lambda r: r.get("ovh_seed"), LOWER, "diff", 0.005),
    ("ovh_leaf_p90", lambda r: r.get("ovh_leaf_p90"), LOWER, "diff", 0.005),
    ("have/useful", per_useful("up_have"), None, "ratio", 0.05),
    ("ack/useful", per_useful("up_ack"), None, "ratio", 0.05),
    ("request/useful", per_useful("up_request"), None, "ratio", 0.05),
    ("integrity/useful", per_useful("up_integrity", "up_signed"), None, "ratio", 0.05),
    ("transport/useful", per_useful("up_transport"), None, "ratio", 0.05),
    ("acks/useful", per_useful("up_acks"), None, "ratio", 0.05),
    ("links_p50", lambda r: r.get("links_p50"), None, "diff", 0.5),
    ("links_p90", lambda r: r.get("links_p90"), None, "diff", 0.5),
    ("links_dropped/min", lambda r: r.get("links_dropped_per_host_min"), LOWER, "diff", 0.1),
    ("eclipsed_share", lambda r: r.get("eclipsed_share"), LOWER, "diff", 0.005),
    ("honest_lag_p99_ms", lambda r: r.get("lag_p99_ms_honest"), LOWER, "ratio", 0.05),
    ("hostile_lag_p99_ms", lambda r: r.get("lag_p99_ms_hostile"), None, "ratio", 0.05),
    ("hostile_delivery", lambda r: r.get("delivery_hostile"), None, "diff", 0.002),
    ("cut_from_seed_share", lambda r: r.get("cut_from_seed_share"), LOWER, "diff", 0.005),
    # Where strong uplinks sit (`Report::capacity_position`).
    ("cap_depth_rho", lambda r: r.get("cap_depth_rho"), LOWER, "diff", 0.05),
    ("cap_depth_rho_startup", lambda r: r.get("cap_depth_rho_startup"), LOWER, "diff", 0.05),
    ("cap_fanout_rho", lambda r: r.get("cap_fanout_rho"), HIGHER, "diff", 0.05),
    ("cap_hops_rho", lambda r: r.get("cap_hops_rho"), LOWER, "diff", 0.05),
    ("seed_nbr_up_p50", lambda r: r.get("seed_nbr_up_p50"), HIGHER, "ratio", 0.10),
    ("depth1_up_p50", lambda r: r.get("depth1_up_p50"), HIGHER, "ratio", 0.10),
    ("top_seed_link_s", lambda r: r.get("top_seed_link_s"), LOWER, "ratio", 0.10),
    ("lag_p99_ms_weak", lambda r: r.get("lag_p99_ms_weak"), LOWER, "ratio", 0.05),
    ("seed_offers_per_min", lambda r: r.get("seed_offers_per_min"), None, "ratio", 0.10),
    ("links_formed/min", lambda r: r.get("links_formed_per_host_min"), None, "ratio", 0.10),
    ("pool_trials/min", lambda r: r.get("pool_trials_per_host_min"), None, "diff", 0.1),
    ("pool_trial_wins", lambda r: r.get("pool_trial_wins"), None, "diff", 0.05),
    ("rejoin_p90_ms", lambda r: r.get("rejoin_p90_ms"), LOWER, "ratio", 0.10),
    # What a defector's lag is to an honest viewer's: under 1, defecting pays.
    ("defector_gain", lambda r: r["lag_p99_ms_hostile"] / r["lag_p99_ms_honest"]
     if r.get("lag_p99_ms_hostile") and r.get("lag_p99_ms_honest") else None, HIGHER, "ratio", 0.05),
    ("exch_rejected", lambda r: r.get("exch_rejected"), None, "diff", 1),
    # What the machine running a job sets as much as the change: shown,
    # marked h, and kept out of the scorecard's totals (HOST_METRICS).
    ("rss_mb", lambda r: r.get("_rss_mb"), LOWER, "diff", 100),
    ("wall_s", lambda r: r.get("_wall_s"), LOWER, "ratio", 0.20),
]

_simdb = None


def simdb():
    """hack/sim-db/simdb.py: the runs' tables, and analyze.py's verdicts
    over them."""
    global _simdb
    if _simdb is None:
        spec = importlib.util.spec_from_file_location("simdb", ROOT / "hack" / "sim-db" / "simdb.py")
        _simdb = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_simdb)
    return _simdb


def analyze():
    """hack/capture/analyze.py, for its per-second verdicts."""
    return simdb().analyze()


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


def build(src, sim, dest, cargo=CARGO):
    """Builds `sim`'s test binary at `src` in target dir `cargo` and copies
    it into `dest`."""
    dest.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, "CARGO_TARGET_DIR": str(cargo), "CARGO_INCREMENTAL": "0"}
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
        build(ROOT, sim, dest, TREE_CARGO)
    return label, dest


def job_id(label, c, seed):
    key = json.dumps([c["sim"], c["test"], sorted(c["env"].items()), seed])
    return f"{label}:{c['name']}:{seed}:{hashlib.sha1(key.encode()).hexdigest()[:12]}"


# A job's peak memory: the kernel keeps a process's peak across exec, so
# one forked from this script starts at the script's own, rows and all.
# GNU time forks the sim from its own small process and reports the sim's.
TIME_TAG = "SIM_SUITE_MAXRSS_KIB"
TIME = ["/usr/bin/time", "-f", f"{TIME_TAG} %M"] if Path("/usr/bin/time").exists() else []


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
                [*TIME, str(bin_dir / c["sim"]), "--exact", c["test"], "--include-ignored", "--nocapture",
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
            kib = usage.ru_maxrss
            if TIME and (m := re.search(rf"^{TIME_TAG} (\d+)$", out, re.M)):
                kib = int(m.group(1))
            row["_rss_mb"] = round(kib / 1024)
            if kib * 1024 > c["mem_gib"] * GiB:
                return job, row, f"peak memory {kib // 1024} MiB past the cell's {c['mem_gib']} GiB"
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
    """A rows file's records by id, but lines that don't parse: a torn
    append's."""
    rows = {}
    if path.exists():
        for line in path.read_text().splitlines():
            try:
                r = json.loads(line)
            except ValueError:
                continue
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


def collapsed(r):
    """Whether a row's run collapsed (analyze.collapsed: COLLAPSED_SECS
    unhealthy seconds in a row lasting past its startup)."""
    v = derived(r)[0]
    return None if v is None else analyze().collapsed(v)


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


def same_run(a, b):
    """Whether rows `a` and `b` ran the same: their traces, and their
    reads' digests where both carry one."""
    reads = a.get("reads_hash") and b.get("reads_hash")
    return a["trace"] == b["trace"] and (not reads or a["reads_hash"] == b["reads_hash"])


def scorecard(cells, a_rows, b_rows, a_label, b_label, show_all=False, traces=True):
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
            if name in HOST_METRICS:
                m = " " if m == " " else "h"
            else:
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
    pairs = [(k[0], same_run(ra, b_rows[k])) for k, ra in a_rows.items()
             if k in b_rows and ra.get("trace") and b_rows[k].get("trace")]
    if traces and pairs:
        same = sum(1 for _, eq in pairs if eq)
        differ = collections.Counter(name for name, eq in pairs if not eq)
        cells_ = ", ".join(f"{name} ({k})" for name, k in differ.most_common(8))
        more = f" and {len(differ) - 8} more" if len(differ) > 8 else ""
        print(f"traces: {same} of {len(pairs)} runs as A's" + (f"; differ in {cells_}{more}" if 0 < same < len(pairs) else ""))


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


def arm_score(cells, base_rows, arm_rows, rank=None):
    """An arm against the base on the same seeds: its collapsed runs and the
    base's, the geometric means over cells of its lag p99 ratio and its
    wire bytes per useful byte, and the guardrails it broke. Ranked for
    overhead (`rank`: --lag-ceiling, --lag-budget), a cell whose median lag
    p99 the base kept under the ceiling breaks one when the arm's isn't, and
    its lag ratios are guarded only by the budget, when one is given."""
    overhead = rank is not None and rank.rank == "overhead"
    guard = rank.lag_budget if overhead else GUARD_RATIO
    ka_all = kb_all = 0
    logs, wires = [], []
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
        w = effect("ratio", pairs("wire/useful"))
        if w is not None:
            wires.append(math.log(w[0]))
        if overhead and (lags := pairs("lag_p99_ms")):
            ma = statistics.median(x for x, _ in lags)
            mb = statistics.median(y for _, y in lags)
            if ma < rank.lag_ceiling <= mb:
                broken.append(f"{c['name']} lag p99 {ma:.0f}->{mb:.0f} ms past {rank.lag_ceiling:.0f}")
        fn, better, kind, tie = get["delivery"]
        if mark(better, kind, tie, effect(kind, pairs("delivery"))) == "-":
            broken.append(f"{c['name']} delivery")
        if ka == 0 and guard is not None:
            for name in ("lag_p99_ms", "lag_p99_ms_weak", "join_p90_ms", "caught_up_p90_ms"):
                e = effect("ratio", pairs(name))
                if e is not None and e[1] > guard:
                    broken.append(f"{c['name']} {name} x{e[0]:.2f}")
        # Defecting must not come to pay more.
        e = effect("ratio", pairs("defector_gain"))
        if e is not None and e[2] < 1:
            broken.append(f"{c['name']} defector_gain x{e[0]:.2f}")
    p99 = math.exp(sum(logs) / len(logs)) if logs else None
    wire = math.exp(sum(wires) / len(wires)) if wires else None
    return {"collapsed": kb_all, "base_collapsed": ka_all, "p99": p99, "wire": wire, "broken": broken,
            "cells": cells_out, "overhead": overhead}


def rank_key(sc):
    """Broken arms last, then fewer collapsed runs, then the lower lag p99
    ratio, or wire per useful byte when ranked for overhead."""
    by = sc["wire"] if sc["overhead"] else sc["p99"]
    return (bool(sc["broken"]), sc["collapsed"], by if by is not None else math.inf)


def ratio_text(x):
    return "-" if x is None else f"x{x:.3f}"


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
        scores = {arm: arm_score(cells, base_rows, union(rows, labels[arm]), a) for arm in arms}
        print(f"\n{title}: {len(arms)} arms")
        print(f"   {'arm':58} collapsed (base)   p99 ratio  wire ratio   guardrails")
        for arm in sorted(arms, key=lambda x: rank_key(scores[x])):
            sc = scores[arm]
            p99, wire = ratio_text(sc["p99"]), ratio_text(sc["wire"])
            print(f"   {','.join(arm):58} {sc['collapsed']:5} ({sc['base_collapsed']:3})   {p99:>9}  {wire:>10}   "
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


# Each sim's output dir knob (hack/sim-db): the run writes its tables there.
OUT_KNOB = {PPSPP: "SIM_OUT", SIM: "STRIMS_SIM_OUT"}
# Finished jobs fold into the invocation's store this many at a time, or
# this long after the first waits.
FOLD_BATCH = 16
FOLD_SECS = 5


class Invocation:
    """One run_cells' store of its jobs' tables (hack/sim-db):
    target/sim-suite/store/<inv>.duckdb, held by a lock while the
    invocation lives, and exported to the cache, source
    suite/<worktree>/<inv>, when it ends. A store no process holds is a
    crashed invocation's, exported when the next one starts."""

    count = itertools.count()

    def __init__(self):
        self.dir = WORK / "store"
        self.dir.mkdir(parents=True, exist_ok=True)
        simdb().recover(self.dir, self.dest)
        for d in (WORK / "out").glob("*"):
            if not (self.dir / f"{d.name}.lock").exists():
                shutil.rmtree(d, ignore_errors=True)
        self.inv = f"{time.strftime('%Y%m%dT%H%M%S')}-{os.getpid()}-{next(Invocation.count)}"
        self.lock = simdb().locked(self.dir / f"{self.inv}.lock")
        self.lock.__enter__()
        self.db = self.dir / f"{self.inv}.duckdb"
        self.out = WORK / "out" / self.inv
        self.src = f"suite/{ROOT.name}/{self.inv}"

    @staticmethod
    def dest(inv):
        return simdb().cache_root() / "cache" / "suite" / ROOT.name / inv

    def close(self):
        if self.db.exists():
            simdb().export(self.db, self.dest(self.inv))
        for p in (self.db, self.db.with_suffix(".duckdb.wal")):
            p.unlink(missing_ok=True)
        shutil.rmtree(self.out, ignore_errors=True)
        self.lock.__exit__(None, None, None)
        (self.dir / f"{self.inv}.lock").unlink(missing_ok=True)


def kept_rows(path):
    """A label's kept runs by id: the lines of rows/<label>.jsonl whose
    tables the cache still holds (a line names its source, `src`); the
    others' jobs run again."""
    return {i: r for i, r in load_rows(path).items() if r.get("src") and simdb().have_src(r["src"])}


def run_cells(cells, runs, slots, mem_gib):
    """Runs every binary of `runs` ({label: (bin_dir, seeds, sim, env)})
    at its seeds for the cells of its sim, with `env` added to every job's
    environment, skipping runs kept (per label, in
    target/sim-suite/rows/<label>.jsonl, a line a run naming the cache
    source of its tables); returns each label's rows by (cell, seed) and
    the failures. Each job writes its tables into an output dir, folded
    into the invocation's store (Invocation) as jobs end; its line is
    written once they are."""
    store = lambda label: WORK / "rows" / f"{label}.jsonl"
    inv = Invocation()
    have = {label: kept_rows(store(label)) for label in runs}
    jobs = []
    for label, (bin_dir, seeds, sim, env) in runs.items():
        for c in cells:
            if c["sim"] != sim:
                continue
            for s in seeds(c):
                jid = job_id(label, c, s)
                if jid in have[label]:
                    continue
                out = inv.out / hashlib.sha1(jid.encode()).hexdigest()[:16]
                out.mkdir(parents=True, exist_ok=True)
                jobs.append((label, bin_dir, c, s, {**env, OUT_KNOB[sim]: str(out)}))
    # A job takes its cell's peak memory in kept rows, a quarter over, and
    # the declared budget until the cell has rows: budgets are worst cases,
    # and holding to them left most cores idle while the heavy cells ran.
    peaks = {}
    for kept in have.values():
        for rec in kept.values():
            if rec.get("_rss_mb"):
                peaks[rec["cell"]] = max(peaks.get(rec["cell"], 0), rec["_rss_mb"])
    need = lambda c: min(c["mem_gib"], 1.25 * peaks[c["name"]] / 1024) if c["name"] in peaks else c["mem_gib"]
    failures = []
    t0 = time.monotonic()
    files = {label: open(store(label), "a") for label in runs}
    pending = []
    first = [None]

    def fold():
        batch = [{"k": job_id(label, c, s), "dir": extra[OUT_KNOB[c["sim"]]], "sim": c["sim"], "test": c["test"],
                  "cell": c["name"], "arm": label, "seed": s, "env": c["env"], "row": row,
                  "wall_s": row.get("_wall_s"), "rss_mb": row.get("_rss_mb")}
                 for (label, _, c, s, extra), row in pending]
        try:
            simdb().fold(inv.db, batch, inv.src)
        except simdb().FoldError:
            # A job without tables: fold the others, and fail it.
            for j in batch:
                try:
                    simdb().fold(inv.db, [j], inv.src)
                except simdb().FoldError as e:
                    failures.append(f"{j['arm']} {j['cell']} seed {j['seed']}: {e}")
                    j["failed"] = True
        for j, ((label, _, c, s, _), row) in zip(batch, pending):
            shutil.rmtree(j["dir"], ignore_errors=True)
            if j.get("failed"):
                continue
            rec = {"id": j["k"], "cell": c["name"], "seed": s, "src": inv.src,
                   **{f: row[f] for f in ("trace", "frames", "bytes") if f in row},
                   "_wall_s": row.get("_wall_s"), "_rss_mb": row.get("_rss_mb")}
            have[label][rec["id"]] = rec
            files[label].write(json.dumps(rec) + "\n")
            files[label].flush()
        pending.clear()
        first[0] = None

    try:
        for done, (job, row, err) in enumerate(run_jobs(jobs, slots, mem_gib, need), 1):
            label, _, c, s, _ = job
            if err:
                failures.append(f"{label} {c['name']} seed {s}: {err}")
            if row is not None:
                pending.append((job, row))
                first[0] = first[0] or time.monotonic()
            if pending and (len(pending) >= FOLD_BATCH or time.monotonic() - first[0] >= FOLD_SECS):
                fold()
            print(f"\r{done}/{len(jobs)} runs, {time.monotonic() - t0:.0f} s", end="", file=sys.stderr)
        if pending:
            fold()
    finally:
        print(file=sys.stderr)
        for f in files.values():
            f.close()
        inv.close()
    want = {(label, c["name"], s): have[label][job_id(label, c, s)]
            for label, (_, seeds, sim, _) in runs.items()
            for c in cells
            if c["sim"] == sim
            for s in seeds(c)
            if job_id(label, c, s) in have[label]}
    got = simdb().rows([(rec["src"], rec["id"]) for rec in want.values()])
    rows = {label: {} for label in runs}
    for (label, cell_name, s), rec in want.items():
        row = got.get((rec["src"], rec["id"]))
        if row is None:
            failures.append(f"{label} {cell_name} seed {s}: no tables in {rec['src']}")
            continue
        rows[label][(cell_name, s)] = row
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
            m = rec.get("_rss_mb") or rec.get("row", {}).get("_rss_mb")
            if m:
                peaks[rec["cell"]] = max(peaks.get(rec["cell"], 0), m / 1024)
    return peaks


def farm_rows(name, cells, knob):
    """Farm run `name`'s rows (its matrices are `cells`, in order) by
    arm: a `--refs` name, or a `--sets` set as its knob's value, each by
    (cell, seed). A matrix's index names each job's tables (`src`), which
    are pulled into the cache from S3 (hack/sim-db) and read as local
    runs' are; a job whose data has expired is left out, to run again."""
    rows = collections.defaultdict(dict)
    want = []
    for i, c in enumerate(cells):
        seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
        index = farm_root() / "target" / "sim-farm" / f"{name}-{i:02d}" / "index.jsonl"
        for rec in load_rows(index).values():
            if rec["test"] != c["test"]:
                sys.exit(f"{name}-{i:02d} ran {rec['test']}, not {c['name']}'s {c['test']}: "
                         "pass the run's --tier and --only")
            if rec.get("src"):
                want.append((c, seed_key, rec))
    gone = set(simdb().pull_sources({rec["src"] for _, _, rec in want}))
    if gone:
        print(f"{name}: {len(gone)} flushes' tables are gone from S3 (a week's): rerun their jobs",
              file=sys.stderr)
    got = simdb().rows([(rec["src"], rec["key"]) for _, _, rec in want if rec["src"] not in gone])
    for c, seed_key, rec in want:
        row = got.get((rec["src"], rec["key"]))
        if row is None:
            continue
        row = {**row, "_rss_mb": rec.get("max_rss_mb"), "_wall_s": rec.get("wall_s")}
        for v in {rec["arm"], rec["env"].get(knob[c["sim"]])} - {None}:
            rows[v][(c["name"], int(rec["env"][seed_key]))] = row
    return rows


def farm_indexes(pending=False):
    """The main checkout's farm-runs/, where farm runs' indexes of their
    cells are (simdb's suite_index reads them too), or its pending/, where
    one waits for the farm to start its run."""
    runs = farm_root() / "target" / "sim-suite" / "farm-runs"
    return runs / "pending" if pending else runs


def farm_index(name, pending=False):
    return farm_indexes(pending) / f"{name}.json"


def farm_started(name, n):
    """Whether the farm has started any of farm run `name`'s `n` matrices:
    its run makes their state dirs, and an estimate doesn't."""
    state = farm_root() / "target" / "sim-farm"
    return any((state / f"{name}-{i:02d}").is_dir() for i in range(n))


def place_index(name, text, started):
    """Writes farm run `name`'s index in place once the farm has started the
    run, apart until then, so a run only quoted, or relaunched under another
    name, leaves none in place."""
    path, other = farm_index(name), farm_index(name, pending=True)
    if not started:
        path, other = other, path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text)
    other.unlink(missing_ok=True)


def settle_indexes(days=7):
    """Moves each pending index whose run the farm has since started (by
    hand, `farm.py run`) into place, and drops one whose run hasn't started
    within `days`."""
    for p in farm_indexes(pending=True).glob("*.json"):
        try:
            n = len(json.loads(p.read_text())["cells"])
            if farm_started(p.stem, n):
                p.replace(farm_index(p.stem))
            elif time.time() - p.stat().st_mtime > days * 86400:
                p.unlink()
        except (OSError, ValueError, KeyError):
            continue


def farm(a, cells):
    # Either one commit with the flag sets as an axis, or several commits
    # as the matrices' arms.
    if a.refs:
        arms = {n.strip(): git("rev-parse", r.strip()) for n, r in (x.split("=", 1) for x in a.refs.split(","))}
        values = list(arms)
        sha = next(iter(arms.values()))
    else:
        sha = git("rev-parse", a.ref)
        arms = {"x": sha}
        values = ([] if a.no_off else ["off"]) + [x.strip() for x in a.sets.split(";") if x.strip()]
    knob = dict(zip((PPSPP, SIM), (k.strip() for k in a.knob.split(","))))
    seeds = [str(s) for s in range(a.first_seed, a.first_seed + a.seeds)]
    # Matrix names are the run's EC2 tag, joined: keep them short, and
    # name the inputs so a rerun adopts the run. --env's settings join them
    # only when given, so runs without keep their names.
    key = [sorted(arms.items()), values, knob, seeds, [c["name"] for c in cells]]
    key = json.dumps(key + ([sorted(a.env)] if a.env else []))
    name = "s" + hashlib.sha1(key.encode()).hexdigest()[:5]
    peaks = {}
    for r in arms.values():
        for cell_, g in peak_gib(r).items():
            peaks[cell_] = max(peaks.get(cell_, 0), g)
    out = WORK / "farm"
    out.mkdir(parents=True, exist_ok=True)
    paths = []
    for i, c in enumerate(cells):
        package, target, _ = BINARIES[c["sim"]]
        seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
        mem = c["mem_gib"]
        if c["name"] in peaks:
            mem = min(mem, math.ceil(1.25 * peaks[c["name"]] * 2) / 2)
        canary = {"SIM_SECS": "2"} if c["sim"] == PPSPP else {"STRIMS_SIM_STREAM_SECS": "5"}
        # An audience that doesn't fit in the canary's stream starts with
        # the seed and stays (the module docs).
        if c["env"].get(ARRIVALS[c["sim"]], "trickle") != "trickle" or c["env"].get(DEPART[c["sim"]]):
            canary |= {ARRIVALS[c["sim"]]: "t0", DEPART[c["sim"]]: "" if c["sim"] == PPSPP else "off"}
        q = json.dumps
        text = "\n".join([
            f"# sim-suite farm {name}: {c['name']}, {c['why']}",
            f"package = {q(package)}",
            f"target = {q(target)}",
            f"test = {q(c['test'])}",
            f"slot_mem_gib = {mem}",
            "[arms]",
            *(f"{n} = {q(r)}" for n, r in arms.items()),
            "[env]",
            *(f"{k} = {q(v)}" for k, v in sorted(c["env"].items())),
            "[axes]",
            *([] if a.refs else [f"{knob[c['sim']]} = {q(values)}"]),
            f"{seed_key} = {q(seeds)}",
            "[canary]",
            *(f"{k} = {q(v)}" for k, v in canary.items()),
            "",
        ])
        path = out / f"{name}-{i:02d}.toml"
        path.write_text(text)
        paths.append(path)
    settle_indexes()
    keep = ("name", "sim", "test", "why")
    # A `--sets` run's arms are its knob's values, which hack/sim-db labels
    # its runs by.
    index = json.dumps({"cells": [{k: c[k] for k in keep} for c in cells],
                        **({} if a.refs else {"knob": knob, "values": values})}) + "\n"
    place_index(name, index, not a.no_run or farm_started(name, len(cells)))
    farm_py = farm_root() / "hack" / "sim-farm" / "farm.py"
    if not a.no_run:
        print(f"farm run {name}: {len(cells)} matrices, {len(values)} arms x {len(seeds)} seeds", file=sys.stderr)
        # One run for the tier: one boot of its instances, and one tail.
        subprocess.run([sys.executable, str(farm_py), "run", *map(str, paths), *shlex.split(a.farm_args)])
        if not farm_started(name, len(cells)):
            place_index(name, index, False)
    rows = farm_rows(name, cells, knob)
    base = a.against.strip() if a.against else values[0]
    if base not in values:
        sys.exit(f"--against {base!r} is none of the sets")
    others = [v for v in values if v != base]
    for v in others:
        print_arm_score(f"{v} against {base} at {sha[:8]}", arm_score(cells, rows[base], rows[v], a))
    for v in others:
        print()
        scorecard(cells, rows[base], rows[v], f"{sha[:8]} {base}", f"{sha[:8]} {v}", a.all)


def print_arm_score(title, sc):
    print(f"\n{title}: collapsed {sc['collapsed']} ({sc['base_collapsed']}), p99 {ratio_text(sc['p99'])}, "
          f"wire/useful {ratio_text(sc['wire'])}, guardrails {'; '.join(sc['broken']) or 'ok'}")
    for cname, ka, kb, n, e in sc["cells"]:
        p = "-" if e is None else f"x{e[0]:.2f} [{e[1]:.2f},{e[2]:.2f}]"
        print(f"   {cname:22} collapsed {ka:>3} -> {kb:>3} of {n:<3} Fisher p={fisher(ka, n, kb, n):.2f}  lag p99 {p}")


def list_cells(a):
    """`list`: each cell of the tier (every tier's without one), with its
    audience and settings, the base envs' common keys left out."""
    common = {**PP_BASE, **SS_BASE}
    for tier in [a.tier] if a.tier else TIERS:
        cells = TIERS[tier]
        if a.only:
            cells = [c for c in cells if c["name"] in set(a.only.split(","))]
        print(f"{tier}: {len(cells)} cells")
        for c in cells:
            env = dict(c["env"])
            arrivals = env.pop(ARRIVALS[c["sim"]], "t0")
            depart = env.pop(DEPART[c["sim"]], "")
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
    knob = dict(zip((PPSPP, SIM), (k.strip() for k in a.knob.split(","))))
    settle_indexes()
    picked = []
    for arg in (a.a, a.b):
        run, sep, arm = arg.rpartition(":")
        if not sep or not run:
            sys.exit(f"{arg!r} isn't RUN:ARM")
        index = farm_index(run)
        run_cells = json.loads(index.read_text())["cells"] if index.exists() else cells
        rows = farm_rows(run, run_cells, knob)
        if arm not in rows:
            sys.exit(f"run {run} has no rows of arm {arm!r} (it has {', '.join(sorted(rows)) or 'none'})")
        picked.append((run_cells, rows[arm]))
    (a_cells, a_rows), (b_cells, b_rows) = picked
    names = {c["name"] for c in a_cells}
    both = [c for c in b_cells if c["name"] in names]
    if a.only:
        both = [c for c in both if c["name"] in set(a.only.split(","))]
    if a.rank == "overhead":
        print_arm_score(f"{a.b} against {a.a}", arm_score(both, a_rows, b_rows, a))
        print()
    scorecard(both, a_rows, b_rows, a.a, a.b, a.all)


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
    fc.add_argument("--sets", help="flag sets, ;-separated, each comma-separated")
    fc.add_argument("--refs", help="commits instead, name=ref,...: the matrices' arms, the first the base")
    fc.add_argument("--knob", default="SIM_EXP,STRIMS_SIM_EXP",
                    help="the knobs that take a set: the ppspp sim's, then strims-sim's")
    fc.add_argument("--ref", default="HEAD", help="the commit the farm builds")
    fc.add_argument("--tier", default="peers", choices=sorted(TIERS))
    fc.add_argument("--seeds", type=int, default=32, help="seeds per cell, every cell alike")
    fc.add_argument("--first-seed", type=int, default=2001)
    fc.add_argument("--against", help="the set or arm the others are scored against (off, or the first ref)")
    fc.add_argument("--only", help="cells, comma-separated")
    fc.add_argument("--all", action="store_true", help="every metric, ties too")
    fc.add_argument("--farm-args", default="", help="more arguments for farm.py run")
    fc.add_argument("--no-run", action="store_true",
                    help="write the matrices and report the rows the farm has, launching nothing")
    fc.add_argument("--no-off", action="store_true", help="run only the sets given, not `off` beside them")
    sc = sub.add_parser("score")
    sc.add_argument("a", metavar="RUN:ARM", help="the base: a farm run's name and one of its arms")
    sc.add_argument("b", metavar="RUN:ARM", help="the arm scored against it")
    sc.add_argument("--tier", default="peers", choices=sorted(TIERS),
                    help="a run's cells when it has no index: the tier it ran")
    sc.add_argument("--only", help="cells, comma-separated (the run's, when it has no index)")
    sc.add_argument("--knob", default="SIM_EXP,STRIMS_SIM_EXP",
                    help="the knobs that took a --sets run's sets: the ppspp sim's, then strims-sim's")
    sc.add_argument("--all", action="store_true", help="every metric, ties too")
    ls = sub.add_parser("list")
    ls.add_argument("--tier", choices=sorted(TIERS), help="one tier (default: every one)")
    ls.add_argument("--only", help="cells, comma-separated")
    for p in (cal, r, rc, fc):
        p.add_argument("--env", action="append", default=[], metavar="KEY=VALUE",
                       help="set for every arm's runs, as part of each cell")
    for p in (rc, fc, sc):
        p.add_argument("--rank", choices=("lag", "overhead"), default="lag",
                       help="rank arms after collapses by lag p99, or by wire bytes per useful byte")
        p.add_argument("--lag-budget", type=float, default=None,
                       help="with --rank overhead: a ratio guarding lag below the ceiling (none by default)")
        p.add_argument("--lag-ceiling", type=float, default=analyze().DELIVERY_TARGET_MS,
                       help="with --rank overhead: the lag p99 (ms) a cell under it must stay under "
                            "(delivery's share of the latency target, analyze.py)")
    a = ap.parse_args()

    if a.cmd == "list":
        list_cells(a)
        return
    cells = TIERS[a.tier]
    if getattr(a, "only", None):
        want = set(a.only.split(","))
        cells = [c for c in cells if c["name"] in want]
    if a.cmd == "score":
        score(a, cells)
        return
    every = dict(kv.split("=", 1) for kv in a.env)
    if every:
        cells = [{**c, "env": {**c["env"], **every}} for c in cells]
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
                                   "the same at seeds N+1..2N", True, traces=False)
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
