# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""suite.py's tiers, cells, metrics, local runs and farm matrices
(hack/sim-suite/suite.py's module docs say what each is for).

  TIERS          {tier: [cell]}, a cell {name, sim, test, env, mem_gib,
                 why, seeds}
  cells(tier, only=None, env=())
                 a tier's cells, `only` the ones named, each with --env's
                 settings (`env`, KEY=VALUE) set
  METRICS        [(name, its value as SQL, better, kind, tie band)], and
                 GUARDRAILS, HOST_METRICS, SHOWN and GUARD_RATIO: what the
                 catalog's scoring macros score (catalog.scoring_sql)
  base_binary(sha, sim, rebuild), tree_binary(sim, base)
                 a sim's test binary at a commit and in the working tree,
                 built once (target/sim-suite/bin/LABEL)
  run_cells(cells, runs, slots, mem_gib) -> (sides, failures)
                 local runs of each cell at each binary's seeds, their
                 tables folded into the cache and indexed: each label's
                 side, {(cell, seed): (k, src)}
  farm_rows(name, cells, knob) -> {arm: side}
                 a farm run's sides by arm, its tables pulled and indexed
  arm_rows(run, cells, arm, index, knob) -> side
                 one arm's side of a farm run, with or without its index
  write_matrices(cells, seeds, arms=None, sets=None, ref="HEAD", knob=None,
                 env=(), question=None, against=None, no_off=False)
                 -> Matrices
                 a farm run's matrices (target/sim-suite/farm/) and its
                 index, as `suite.py farm --no-run` writes them

Its state is the checkout's target/sim-suite (WORK: binaries, kept rows,
an invocation's store, matrices) and the main checkout's (farm_root():
the farm's state and the farm runs' indexes). stdlib alone: the store and
the catalog are simdb.py's and catalog.py's (simdb(), catalog()), and an
invocation's store folds and exports in the duckdb module where it
imports (simlab.store.installed: under the engine), else in the CLI.
"""

import collections
import concurrent.futures as cf
import dataclasses
import hashlib
import importlib.util
import itertools
import json
import math
import os
import re
import shutil
import subprocess
import sys
import threading
import time
from pathlib import Path

import simlab
from simlab import store

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

# The knobs that take a race's flags and a farm run's sets: the ppspp sim's,
# then strims-sim's.
DEFAULT_KNOB = "SIM_EXP,STRIMS_SIM_EXP"

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


# The streams the sign tiers run (strims_wan::live's encoders): audio alone
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

# The measure tier's cells, by mechanism: robust's (steady, overload, the
# seed's uplink halved, thin relays, joins and departures, jitter), then
# peers' (mobile viewers).
MEASURE_ROBUST = ["star-15-10", "star-31-5", "halve-15-10", "halve-31-10", "thin-15-10", "star-15-10-event",
                  "star-15-10-end-abrupt", "jitter-15-10"]
MEASURE_PEERS = ["wan-mobile"]
# The logs a measurement keeps, by sim: the forecast log and its truth (the
# chunks each node stored), and the scheduler's park and event logs. The
# tables go to each job's output dir, as every job's do.
MEASURE_LOGS = {PPSPP: {"SIM_FORECAST": "1", "SIM_SPEC": "1"},
                SIM: {"STRIMS_SIM_FORECAST": "1", "STRIMS_SIM_SPEC": "1"}}
MEASURE_SEEDS = 8
# The logs hold a row per neighbour per chunk: a run's memory grows with
# its viewers and its stream's length.
MEASURE_MEM_GIB = 3


def measure_tier():
    """The cells a forecast or request change is measured on, with every
    log on (MEASURE_LOGS) at MEASURE_SEEDS seeds each. The logs change no
    run, and the cells keep their tiers' names."""
    robust = {c["name"]: c for c in TIERS["robust"]}
    peers = {c["name"]: c for c in TIERS["peers"]}
    return [{**c, "env": {**c["env"], **MEASURE_LOGS[c["sim"]]}, "mem_gib": max(c["mem_gib"], MEASURE_MEM_GIB),
             "seeds": MEASURE_SEEDS, "why": f"{c['why']}, every log on"}
            for c in [robust[n] for n in MEASURE_ROBUST] + [peers[n] for n in MEASURE_PEERS]]


TIERS["measure"] = measure_tier()


def cells(tier, only=None, env=()):
    """Tier `tier`'s cells, but for those `only` names when given, each
    with `env` (--env's settings, KEY=VALUE) set as part of it, so the rows
    of one setting never stand for another's."""
    out = TIERS[tier]
    if only:
        want = set(only)
        out = [c for c in out if c["name"] in want]
    every = dict(kv.split("=", 1) for kv in env)
    if every:
        out = [{**c, "env": {**c["env"], **every}} for c in out]
    return out


# Metrics: (name, its value as SQL, better: "lower", "higher" or None,
# kind: "ratio" (geometric) or "diff" (absolute), tie band). The SQL reads
# a job's row (r: the catalog's `rows`, a column a field), its verdict and
# run metrics (v: `verdicts`, analyze.py's) and its host's numbers (j:
# `jobs`); a field the catalog's rows lack reads NULL, and NULL is no
# value (simlab.catalog's job_metrics, scoring.sql's macros).
LOWER, HIGHER = "lower", "higher"


def share(num, den):
    """`num` over `den`, none where `den` is 0."""
    return f"CASE WHEN {den} <> 0 THEN coalesce({num}, 0) / ({den}) END"


def per_useful(*fields):
    """The row's `fields` summed, per byte of the stream the honest viewers
    read (`useful_bytes`); none without all of them."""
    return f"CASE WHEN r.useful_bytes <> 0 THEN ({' + '.join(f'r.{f}' for f in fields)}) / r.useful_bytes END"


# The mean hops to the seed over chains that reach it (ppspp sim).
DEPTH_MEAN = share(" + ".join(f"{k} * coalesce(r.depth_{k}, 0)" for k in range(1, 7)),
                   " + ".join(f"coalesce(r.depth_{k}, 0)" for k in range(1, 7)))

METRICS = [
    # A row with hostile viewers scores its honest ones'.
    ("lag_p50_ms", "coalesce(r.lag_p50_ms_honest, r.lag_p50_ms)", LOWER, "ratio", 0.05),
    ("lag_p99_ms", "coalesce(r.lag_p99_ms_honest, r.lag_p99_ms)", LOWER, "ratio", 0.05),
    ("unhealthy", "v.unhealthy", LOWER, "diff", 0.03),
    # The run's health over time (analyze.run_metrics).
    ("collapses", "v.collapses", LOWER, "diff", 0.5),
    ("flaps_per_min", "v.flaps_per_min", LOWER, "diff", 0.2),
    ("unhealthy_dwell_p90_s", "v.unhealthy_dwell_p90_s", LOWER, "diff", 2),
    ("onset_after_drop_s", "v.onset_after_drop_s", HIGHER, "diff", 2),
    ("recover_after_rise_s", "v.recover_after_rise_s", LOWER, "diff", 2),
    ("excess_lag_area", "v.excess_lag_area_ms_s", LOWER, "ratio", 0.10),
    ("osc_amp_ms", "v.osc_amp_ms", LOWER, "ratio", 0.10),
    ("delivery", "coalesce(r.delivery_honest, r.delivery)", HIGHER, "diff", 0.002),
    ("timeouts", "r.timeouts", LOWER, "ratio", 0.10),
    # timeouts' kinds: requests that timed out unanswered, and speculative
    # ones that expired or that their holder answered NOT_YET (the rows'
    # spec_expired and spec_not_yet are the park counters). Rows from
    # before speculative requests lack them.
    ("request_timeouts", "r.request_timeouts", LOWER, "ratio", 0.10),
    ("timeouts_spec_expired", "r.timeouts_spec_expired", LOWER, "ratio", 0.10),
    ("timeouts_spec_not_yet", "r.timeouts_spec_not_yet", LOWER, "ratio", 0.10),
    ("wasted/read", share("r.wasted", "r.chunks_read"), LOWER, "diff", 0.01),
    ("seed_answer_share", "r.seed_pull_share", None, "diff", 0.02),
    ("viewer_util_p50", "r.viewer_util_p50", None, "diff", 0.02),
    ("viewer_util_p90", "r.viewer_util_p90", None, "diff", 0.02),
    ("relay_share", "r.relay_share", None, "diff", 0.03),
    ("depth_mean", DEPTH_MEAN, None, "ratio", 0.05),
    ("depth_cycle", "r.depth_cycle", LOWER, "diff", 0.01),
    ("depth_pull_rooted", "r.depth_pull_rooted", LOWER, "diff", 0.01),
    ("fanout_p50", "r.fanout_p50", None, "ratio", 0.10),
    ("fanout_p90", "r.fanout_p90", None, "ratio", 0.10),
    ("parent_changes_per_min", "r.parent_changes_per_min", LOWER, "ratio", 0.10),
    ("relay_declines", share("r.relay_rejected", "coalesce(r.relay_rejected, 0) + coalesce(r.relay_accepted, 0)"),
     None, "diff", 0.03),
    ("declines_idle", "r.declines_idle", LOWER, "diff", 0.05),
    ("declined_wrongly", share("r.declined_wrongly", "coalesce(r.declined, 0)"), LOWER, "diff", 0.05),
    # A median the row has as a field, else its forwarders' (others') list's
    # middle (p10, p50, p90).
    ("meter_relays_p50", "CASE WHEN r.meter_relays_p50 <> 0 THEN r.meter_relays_p50 ELSE r.meter_forwarders[2] END",
     None, "ratio", 0.10),
    ("meter_others_p50", "CASE WHEN r.meter_others_p50 <> 0 THEN r.meter_others_p50 ELSE r.meter_others[2] END",
     None, "ratio", 0.10),
    ("holder_short", "r.holder_short", LOWER, "diff", 0.03),
    # Each viewer from its own join (with arrivals or departures): to its
    # first read or frame, to its first under the delivery target, the
    # viewers that never did.
    ("join_p50_ms", "r.join_p50_ms", LOWER, "ratio", 0.10),
    ("join_p90_ms", "r.join_p90_ms", LOWER, "ratio", 0.10),
    ("caught_up_p50_ms", "r.caught_up_p50_ms", LOWER, "ratio", 0.10),
    ("caught_up_p90_ms", "r.caught_up_p90_ms", LOWER, "ratio", 0.10),
    ("caught_up_never", "r.caught_up_never", LOWER, "diff", 0.5),
    # The viewers that stayed through departures: their worst lag, their lag
    # p90's excess over the delivery target summed, the time back under it,
    # those left with no neighbour; and how soon a child noticed its parent
    # gone (strims-sim).
    ("depart_lag_peak_ms", "r.depart_lag_peak_ms", LOWER, "ratio", 0.10),
    ("depart_excess_area", "r.depart_excess_lag_area_ms_s", LOWER, "ratio", 0.10),
    ("depart_recover_ms", "r.depart_recover_ms", LOWER, "diff", 2000),
    ("depart_stranded", "r.depart_stranded", LOWER, "diff", 0.5),
    ("parent_lost_noticed_p90_ms", "r.parent_lost_noticed_p90_ms", LOWER, "ratio", 0.10),
    # The honest viewers' lag p99 over their seconds of startup, and of
    # steady state (the verdict's phases: verdict.phase_lags).
    ("lag_p99_startup_ms", "v.lag_p99_startup_ms", LOWER, "ratio", 0.10),
    ("lag_p99_steady_ms", "v.lag_p99_steady_ms", LOWER, "ratio", 0.05),
    ("signed_lag_p99_ms", "r.signed_lag_p99_ms", LOWER, "ratio", 0.05),
    ("wire/media", "r.wire_per_media", LOWER, "ratio", 0.02),
    # Where the uplinks' bytes go (the sims' `up_*` ledger), per byte read.
    ("wire/useful", "r.wire_per_useful", LOWER, "ratio", 0.02),
    ("packets/useful", per_useful("packets_up"), LOWER, "ratio", 0.02),
    ("dup/useful", per_useful("dup_bytes"), LOWER, "diff", 0.002),
    ("ovh_seed", "r.ovh_seed", LOWER, "diff", 0.005),
    ("ovh_leaf_p90", "r.ovh_leaf_p90", LOWER, "diff", 0.005),
    ("have/useful", per_useful("up_have"), None, "ratio", 0.05),
    ("state/useful", per_useful("up_state"), None, "ratio", 0.05),
    ("ack/useful", per_useful("up_ack"), None, "ratio", 0.05),
    ("request/useful", per_useful("up_request"), None, "ratio", 0.05),
    ("integrity/useful", per_useful("up_integrity", "up_signed"), None, "ratio", 0.05),
    ("transport/useful", per_useful("up_transport"), None, "ratio", 0.05),
    ("acks/useful", per_useful("up_acks"), None, "ratio", 0.05),
    ("links_p50", "r.links_p50", None, "diff", 0.5),
    ("links_p90", "r.links_p90", None, "diff", 0.5),
    ("links_dropped/min", "r.links_dropped_per_host_min", LOWER, "diff", 0.1),
    ("eclipsed_share", "r.eclipsed_share", LOWER, "diff", 0.005),
    ("honest_lag_p99_ms", "r.lag_p99_ms_honest", LOWER, "ratio", 0.05),
    ("hostile_lag_p99_ms", "r.lag_p99_ms_hostile", None, "ratio", 0.05),
    ("hostile_delivery", "r.delivery_hostile", None, "diff", 0.002),
    ("cut_from_seed_share", "r.cut_from_seed_share", LOWER, "diff", 0.005),
    # Where strong uplinks sit (`Report::capacity_position`).
    ("cap_depth_rho", "r.cap_depth_rho", LOWER, "diff", 0.05),
    ("cap_depth_rho_startup", "r.cap_depth_rho_startup", LOWER, "diff", 0.05),
    ("cap_fanout_rho", "r.cap_fanout_rho", HIGHER, "diff", 0.05),
    ("cap_hops_rho", "r.cap_hops_rho", LOWER, "diff", 0.05),
    ("seed_nbr_up_p50", "r.seed_nbr_up_p50", HIGHER, "ratio", 0.10),
    ("depth1_up_p50", "r.depth1_up_p50", HIGHER, "ratio", 0.10),
    ("top_seed_link_s", "r.top_seed_link_s", LOWER, "ratio", 0.10),
    ("lag_p99_ms_weak", "r.lag_p99_ms_weak", LOWER, "ratio", 0.05),
    ("seed_offers_per_min", "r.seed_offers_per_min", None, "ratio", 0.10),
    ("links_formed/min", "r.links_formed_per_host_min", None, "ratio", 0.10),
    ("pool_trials/min", "r.pool_trials_per_host_min", None, "diff", 0.1),
    ("pool_trial_wins", "r.pool_trial_wins", None, "diff", 0.05),
    ("rejoin_p90_ms", "r.rejoin_p90_ms", LOWER, "ratio", 0.10),
    # What a defector's lag is to an honest viewer's: under 1, defecting pays.
    ("defector_gain", "CASE WHEN r.lag_p99_ms_hostile <> 0 AND r.lag_p99_ms_honest <> 0 "
     "THEN r.lag_p99_ms_hostile / r.lag_p99_ms_honest END", HIGHER, "ratio", 0.05),
    ("exch_rejected", "r.exch_rejected", None, "diff", 1),
    # What the machine running a job sets as much as the change: shown,
    # marked h, and kept out of the scorecard's totals (HOST_METRICS).
    ("rss_mb", "j.rss_mb", LOWER, "diff", 100),
    ("wall_s", "j.wall_s", LOWER, "ratio", 0.20),
]
HOST_METRICS = ("rss_mb", "wall_s")
# The metrics a scorecard shows in every cell, ties too.
SHOWN = ("lag_p50_ms", "lag_p99_ms", "unhealthy", "delivery")
# arm_score's guardrails, in the order it checks them: "worse" breaks an
# arm marked worse in a cell; "lo" one whose ratio interval lies past
# GUARD_RATIO in a cell the base never collapsed (past --lag-budget when
# ranked for overhead, and none without one); "hi" one whose ratio
# interval lies under 1. More collapsed runs with a Fisher p under 0.05
# break an arm too.
GUARDRAILS = {"delivery": "worse", "lag_p99_ms": "lo", "lag_p99_ms_weak": "lo", "join_p90_ms": "lo",
              "caught_up_p90_ms": "lo", "defector_gain": "hi"}
GUARD_RATIO = 1.10


# simdb.py's module, when a test sets its own.
_simdb = None


def simdb():
    """hack/sim-db/simdb.py (simlab.simdb()): the runs' tables, and
    analyze.py's verdicts over them."""
    return _simdb or simlab.simdb()


def analyze():
    """hack/capture/analyze.py, for its per-second verdicts."""
    return simdb().analyze()


def git(*args, cwd=ROOT):
    return subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True, text=True).stdout.strip()


_catalog = None


def catalog():
    """hack/sim-db/catalog.py: the sims' results catalog, which indexes the
    runs' tables and holds the scoring macros."""
    global _catalog
    if _catalog is None:
        spec = importlib.util.spec_from_file_location("catalog", ROOT / "hack" / "sim-db" / "catalog.py")
        _catalog = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(_catalog)
        _catalog._simdb = simdb()
    return _catalog


def index_catalog():
    """Brings the catalog up to date with the cache (catalog.index), so it
    holds every run this script scores. An index that fails leaves the
    catalog as it was: the runs it lacks then fail, and their rows stay
    kept for the next invocation to score."""
    try:
        catalog().index(quiet=True)
    except SystemExit as e:
        print(f"the catalog's index failed, scoring what it holds: {e}", file=sys.stderr)


def q(s):
    return simdb().q(s)


# --- local runs -------------------------------------------------------------


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
    `need_gib`. Yields, each time jobs end, the (job, row, error) of
    every job that ended since the last yield."""
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
        waiting = [ex.submit(one, j) for j in order]
        while waiting:
            ended, _ = cf.wait(waiting, return_when=cf.FIRST_COMPLETED)
            yield [f.result() for f in waiting if f in ended]
            waiting = [f for f in waiting if f not in ended]


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


# Each sim's output dir knob (hack/sim-db): the run writes its tables there.
OUT_KNOB = {PPSPP: "SIM_OUT", SIM: "STRIMS_SIM_OUT"}
# Finished jobs fold into the invocation's store as they end, those that
# ended while the last fold ran together, at most this many a transaction.
FOLD_BATCH = 16


class Invocation:
    """One run_cells' store of its jobs' tables (hack/sim-db):
    target/sim-suite/store/<inv>.duckdb, held by a lock while the
    invocation lives, and exported to the cache, source
    suite/<worktree>/<inv>, when it ends. A store no process holds is a
    crashed invocation's, exported when the next one starts. While it
    lives, the store's scripts run in the duckdb module where it imports
    (simlab.store.installed)."""

    count = itertools.count()

    def __init__(self):
        self.dir = WORK / "store"
        self.dir.mkdir(parents=True, exist_ok=True)
        with store.installed():
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
        self.executor = store.installed()
        self.executor.__enter__()

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
        self.executor.__exit__(None, None, None)


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
    source of its tables); returns each label's side, its runs' jobs by
    (cell, seed), and the failures. Each job writes its tables into an
    output dir, folded into the invocation's store (Invocation) as soon as
    the job ends and then removed (a failed job's at once), so an
    invocation holds the CSV of the jobs running and folding alone; its
    line is written once they are folded. The invocation's tables go to
    the cache, and the catalog indexes them before a side is made of
    them: a run whose rows the catalog lacks fails."""
    store = lambda label: WORK / "rows" / f"{label}.jsonl"
    (WORK / "rows").mkdir(parents=True, exist_ok=True)
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

    def fold(pending):
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

    try:
        done = 0
        for ended in run_jobs(jobs, slots, mem_gib, need):
            pending = []
            for job, row, err in ended:
                label, _, c, s, extra = job
                if err:
                    failures.append(f"{label} {c['name']} seed {s}: {err}")
                if row is not None:
                    pending.append((job, row))
                else:
                    shutil.rmtree(extra[OUT_KNOB[c["sim"]]], ignore_errors=True)
            for i in range(0, len(pending), FOLD_BATCH):
                fold(pending[i:i + FOLD_BATCH])
            done += len(ended)
            print(f"\r{done}/{len(jobs)} runs, {time.monotonic() - t0:.0f} s", end="", file=sys.stderr)
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
    index_catalog()
    got = held([rec["id"] for rec in want.values()])
    rows = {label: {} for label in runs}
    for (label, cell_name, s), rec in want.items():
        if rec["id"] not in got:
            failures.append(f"{label} {cell_name} seed {s}: no tables in {rec['src']}")
            continue
        rows[label][(cell_name, s)] = (rec["id"], rec["src"])
    return rows, failures


def held(ks):
    """The jobs among `ks` whose rows the catalog holds."""
    if not ks:
        return set()
    rows = catalog().lake(f"SELECT k FROM rows WHERE k IN (SELECT unnest([{', '.join(q(k) for k in ks)}]));")
    return {r["k"] for r in rows}


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


def farm_jobs(name, cells):
    """Farm run `name`'s jobs (its matrices are `cells`, in order): (cell,
    seed, index line) each. A matrix's index names each job's tables
    (`src`), which are pulled into the cache from S3 (hack/sim-db) and
    indexed by the catalog, as local runs' are; a job whose data has
    expired is left out, to run again."""
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
    index_catalog()
    return [(c, int(rec["env"][seed_key]), rec) for c, seed_key, rec in want if rec["src"] not in gone]


def by_arm(jobs, knob):
    """`jobs` (farm_jobs') as sides by arm, each its jobs by (cell, seed),
    the arm as hack/sim-db names it (simdb.arm_label): with `knob` (a
    `--sets` run's, by sim) a set, the value its job's env gives the knob,
    else its matrix's arm, a `--refs` name; and the arms that would hold
    two jobs at one cell and seed, which that naming doesn't keep apart."""
    rows, clash = collections.defaultdict(dict), set()
    for c, seed, rec in jobs:
        arm = simdb().arm_label(knob, c["sim"], rec["arm"], rec["env"])
        if rows[arm].setdefault((c["name"], seed), (rec["key"], rec["src"]))[0] != rec["key"]:
            clash.add(arm)
        rows[arm][(c["name"], seed)] = (rec["key"], rec["src"])
    return rows, clash


def farm_rows(name, cells, knob):
    """Farm run `name`'s sides (its matrices are `cells`, in order) by
    arm, each its jobs by (cell, seed): by_arm's, `knob` a `--sets` run's (its index's)
    and None for a `--refs` run's. An arm holding two jobs at one cell and
    seed is refused."""
    rows, clash = by_arm(farm_jobs(name, cells), knob)
    if clash:
        sys.exit(f"{name}: arm {', '.join(sorted(clash))} holds two jobs at one cell and seed: "
                 + ("its knob " + ", ".join(knob.values()) + " doesn't" if knob else "its matrices' arms don't")
                 + " name the run's arms")
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


def place_index(name, index, started):
    """Writes farm run `name`'s index in place once the farm has started the
    run, apart until then, so a run only quoted, or relaunched under another
    name, leaves none in place."""
    path, other = farm_index(name), farm_index(name, pending=True)
    if not started:
        path, other = other, path
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(index) + "\n")
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


@dataclasses.dataclass
class Matrix:
    """One farm matrix: its file, its cell's name and the env it runs."""
    path: str
    cell: str
    env: dict


@dataclasses.dataclass
class Matrices:
    """A farm run's matrices: its name (the run's EC2 tag, s + 5 hex), its
    matrices in cell order, and its index (farm-runs/<name>.json)."""
    name: str
    matrices: list
    index: dict


def write_matrices(cells, seeds, arms=None, sets=None, ref="HEAD", knob=None, env=(), question=None,
                   against=None, no_off=False):
    """Writes a farm run's matrices, target/sim-suite/farm/RUN-NN.toml, a
    matrix a cell of `cells` in order, whose axes are `seeds` (each cell
    alike) and, for a run of sets, the knob, and the run's index, kept
    apart (pending/) until the farm starts the run. The arms are commits
    (`arms`, {name: ref}, the first the base: the matrices' arms) or one
    commit (`ref`) with the knob (`knob`, {sim: knob}, DEFAULT_KNOB's by
    default) at `off`, unless `no_off`, and at each of `sets` (each a set,
    its flags comma-separated). The others are scored against `against`,
    by default the first. `env` is --env's settings as given (KEY=VALUE),
    which `cells` carry (cells(env=)), and names the run with them;
    `question` is what the run asks. Exits when `against` is none of the
    arms, its matrices written."""
    # Either one commit with the flag sets as an axis, or several commits
    # as the matrices' arms.
    refs = bool(arms)
    if refs:
        arms = {n: git("rev-parse", r) for n, r in arms.items()}
        values = list(arms)
    else:
        arms = {"x": git("rev-parse", ref)}
        values = ([] if no_off else ["off"]) + list(sets)
    knob = knob or dict(zip((PPSPP, SIM), DEFAULT_KNOB.split(",")))
    knob = {sim: knob[sim] for sim in (PPSPP, SIM) if sim in knob}
    seeds = [str(s) for s in seeds]
    # Matrix names are the run's EC2 tag, joined: keep them short, and
    # name the inputs so a rerun adopts the run. --env's settings join them
    # only when given, so runs without keep their names. A cell counts by
    # its settings as well as its name: the browsers tier's cells keep the
    # robust tier's names.
    key = [sorted(arms.items()), values, knob, seeds, [[c["name"], sorted(c["env"].items())] for c in cells]]
    key = json.dumps(key + ([sorted(env)] if env else []))
    name = "s" + hashlib.sha1(key.encode()).hexdigest()[:5]
    peaks = {}
    for r in arms.values():
        for cell_, g in peak_gib(r).items():
            peaks[cell_] = max(peaks.get(cell_, 0), g)
    out = WORK / "farm"
    out.mkdir(parents=True, exist_ok=True)
    matrices, mems = [], []
    for i, c in enumerate(cells):
        package, target, _ = BINARIES[c["sim"]]
        seed_key = "SIM_SEED" if c["sim"] == PPSPP else "STRIMS_SIM_SEED"
        mem = c["mem_gib"]
        if c["name"] in peaks:
            mem = min(mem, math.ceil(1.25 * peaks[c["name"]] * 2) / 2)
        mems.append(mem)
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
            *([] if refs else [f"{knob[c['sim']]} = {q(values)}"]),
            f"{seed_key} = {q(seeds)}",
            "[canary]",
            *(f"{k} = {q(v)}" for k, v in canary.items()),
            "",
        ])
        path = out / f"{name}-{i:02d}.toml"
        path.write_text(text)
        matrices.append(Matrix(str(path), c["name"], c["env"]))
    settle_indexes()
    keep = ("name", "sim", "test", "why", "env")
    base = values[0] if against is None else against
    if base not in values:
        sys.exit(f"--against {base!r} is none of the sets")
    # A `--sets` run's arms are its knob's values, which hack/sim-db labels
    # its runs by (arm_label).
    index = {
        "cells": [{**{k: c[k] for k in keep}, "mem_gib": mem} for c, mem in zip(cells, mems)],
        "arms": arms, "seeds": [int(s) for s in seeds], "base": base,
        **({"question": question} if question else {}),
        **({} if refs else {"knob": knob, "values": values}),
    }
    place_index(name, index, farm_started(name, len(cells)))
    return Matrices(name, matrices, index)


def arm_rows(run, cells, arm, index, knob):
    """Arm `arm`'s side of farm run `run` (its matrices are `cells`), its
    jobs by (cell, seed): by its `index`, a `--sets` run's arm is a set (its
    index's knob's value) and a `--refs` run's a name. A run without an
    index is read both ways, the sets by `knob`, each way only where it
    keeps the jobs of a cell and seed apart; an arm both ways name, with
    other rows, is refused as ambiguous."""
    if index:
        if knob and index.get("knob") and knob != index["knob"]:
            sys.exit(f"run {run}'s sets are its index's knob ({', '.join(index['knob'].values())}), "
                     f"not --knob's ({', '.join(knob.values())})")
        rows = farm_rows(run, cells, index.get("knob"))
        if arm not in rows:
            kind = "sets" if index.get("knob") else "--refs names"
            sys.exit(f"run {run} has no rows of arm {arm!r} (its arms are {kind}: "
                     f"{', '.join(sorted(rows)) or 'none'})")
        return rows[arm]
    jobs = farm_jobs(run, cells)
    named, have = [], set()
    for k in (knob or dict(zip((PPSPP, SIM), DEFAULT_KNOB.split(","))), None):
        rows, clash = by_arm(jobs, k)
        have |= set(rows) - clash
        if arm in rows and arm not in clash:
            named.append(rows[arm])
    if not named:
        sys.exit(f"run {run} has no rows of arm {arm!r} (it has {', '.join(sorted(have)) or 'none'})")
    if len(named) > 1 and named[0] != named[1]:
        sys.exit(f"run {run} has no index, and {arm!r} names both a set (--knob's value) and a --refs name, "
                 "with other rows")
    return named[0]
