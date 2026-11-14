#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Reads ppspp captures of a swarm: what its push trees decided, and why;
and judges runs second by second: whether their trees failed and their
delivery or latency collapsed.

  hack/capture/analyze.py reduce DUMP|DIR... [--from S] [--until S] [--all]
                                             [--bin PATH] > run.jsonl
  hack/capture/analyze.py seed   run.jsonl [--node NAME] [--from S] [--until S]
  hack/capture/analyze.py queues run.jsonl NODE [--from S] [--until S] [--step S]
  hack/capture/analyze.py links  run.jsonl NODE PEER... [--from S] [--until S]
                                 [--step S]
  hack/capture/analyze.py paths  run.jsonl NODE [PEER...] [--from S] [--until S]
                                 [--step S]
  hack/capture/analyze.py prunes run.jsonl [--node NAME] [--from S] [--until S]
  hack/capture/analyze.py pulls  run.jsonl [--source NAME] [--from S] [--until S]
  hack/capture/analyze.py node   run.jsonl NODE [--from S] [--until S]
  hack/capture/analyze.py answers run.jsonl [--node NAME] [--from S] [--until S]
  hack/capture/analyze.py gate   run.jsonl [--node NAME] [--from S] [--until S]
  hack/capture/analyze.py holders run.jsonl [--node NAME] [--holder NAME]
                                  [--from S] [--until S]
  hack/capture/analyze.py diverge  a.jsonl b.jsonl [--source NAME] [--from S]
                                  [--until S]
  hack/capture/analyze.py bytes  run.jsonl [--seed NAME] [--node NAME] [--step S]
                                 [--from S] [--until S]

`reduce` writes the rows the other commands read: `ppspp-capture swarm
--labels --start` over every node's dumps (the capture crate's swarm module
documents the rows and options), one JSON object per line, with nodes and
peers by their capture labels ("seed", "v3", "testbed-2"), times in ms
since the capture's start, durations in ns and rates in bytes per second.
It keeps the decisions, samples, DATA, REQUEST and STREAM_* rows (all rows
with `--all`) from `--from` to `--until` seconds after the start. Build
the tool with `cargo build --release -p strims-ppspp-capture`; `--bin` or
PPSPP_CAPTURE names another.

The other commands print tables about one question each, `--from` and
`--until` in seconds:

  seed    the seed's first wave, budget and window, its upload meter beside
          the transport's view (the model's uplink in a sim), its
          admissions per second by rule, and its holders coming and going
  queues  per sample, what sits below one node's scheduler, summed over its
          links: the runner's data queue, vnic's link buffers, the driver's
          write buffers, and the transport (the sim's model, TCP_INFO, or
          a browser's buffered amounts)
  links   the same per link, for the peers named
  paths   vnic's path measure of each of a node's links (or the peers
          named) per sample, beside what the link's queues held: the round
          trip, its base and excess, the bytes in flight and queued, the
          delivery rate and the queued bytes' delay; and the bytes below
          vnic not yet sent (vnic's buffer, the driver's write buffers and
          the transport's unsent bytes) with the model's uplink queue in a
          sim. Then each link's medians
  prunes  stale pushes and push waits at a node, the prunes by rule with
          their numbers, and the misses and pushes behind the first prune
  pulls   where the viewers' requests went (the source or other viewers),
          the request windows behind them, and request timeouts
  node    one node's decisions and traffic per second, its first admissions
          with their numbers, and its upload meter
  answers per second, what a node (the seed) sent: chunks new to it, pushes,
          answers, the copies of each fresh chunk it sent as answers, and
          how long pushes and answers waited in its queue
  gate    per second, a node's answer pace (the seed's): its rate and the
          queueing delay it read, requests in, pushes and answers out
  holders per holder of a node (the seed), per second while it holds: the
          copy's rate against what the node wrote to it, the pushes queued
          for it and its connection's window, vnic's measure of the link
          (round trip, base, recent least, queued), what it relayed, its
          pushes dropped, and its children's prunes of it; then how often
          holders changed, and why
  diverge two runs of one cell side by side per second (the seed's copies,
          busy share, pushes and answers, the viewers' requests to it, hits,
          misses and prunes), the first second they part (on-time pushes or
          the seed's answers twice the other's), and each run's decisions
          that the other lacks in the 2 s before it
  bytes   where the ppspp frames' bytes go (`reduce --all` rows): per role
          (the seed, relays that sent pushes, leaves) and direction, the
          KB/s per node, each class's share (DATA's payload and headers,
          the message types, the channel headers), the frames' rate, mean
          size, messages each and sizes; `--node` per peer of one node,
          `--step` per interval. vnic, vpn and the transport's bytes
          aren't in a capture: the sims' rows count them

Runs, second by second (no capture needed):

  hack/capture/analyze.py series   RUNS... [--where K=V...] [--every S]
  hack/capture/analyze.py collapse RUNS... [--where K=V...]
  hack/capture/analyze.py map      RUNS... [--where K=V...] [--group K,...]
  hack/capture/analyze.py pair     RUNS... --a K=V[,K=V] --b K=V[,K=V]...
                                   [--where K=V...] [--ignore K,...]
  hack/capture/analyze.py track    RUNS... [--where K=V...] [--group K,...]
                                   [--runs]
  hack/capture/analyze.py delays   RUNS... [--where K=V...] [--group K,...]

RUNS are farm matrix dirs (`target/sim-farm/<matrix>`: a test matrix's
index.jsonl, whose jobs' tables hack/sim-db pulls into its cache and
reads, the rows and series there; a testbed matrix's rows.jsonl and run
dirs), index.jsonl or rows.jsonl files, or testbed run dirs
(`hack/testbed/run.sh`'s out dir). A sim run's series is its tables'
(`seconds` and `viewer_seconds`, which `Report::series_data` documents), with
`STRIMS_SIM_SERIES=1` (strims-sim, where `calibration_star` always has
it) or `SIM_SERIES=1` (the ppspp sim's `sim_job`); a testbed run's comes
from its nodes' lines. `--where` keeps the runs whose env (or row field) has
the value (a blank value matches a setting the run lacks); keys match
with or without the STRIMS_SIM_CAL_, STRIMS_SIM_ or SIM_ prefix.

  series    per second: viewers present, on-time pushes against the chunks
            the viewers needed, the median viewer's delivery, the swarm's
            lag, prunes, parents, and the seed's meter, uplink, busy share,
            copies and share of its chunks sent as pushes; with rows that
            have them, the overhead the seed and the mean viewer wrote
  collapse  one line per run: its verdicts, onsets, course and feasibility
  map       per cell (the runs that share every setting but the seed): the
            share of runs whose trees failed, delivery collapsed and latency
            collapsed, the share absorbing, the median share of the stream's
            seconds that weren't healthy (any verdict's condition held), the
            median onsets, and whether the cell is feasible
  pair      arms compared seed by seed within each cell: an arm is the runs
            whose env (or row) has every K=V of its --a or --b (several --b
            compare each with --a), and a cell is every other setting but
            the seed and the keys --ignore names (a setting one arm lacks
            reads as blank). Per cell and --b: the pairs, each arm's median
            unhealthy share and absorbing runs, the median per-seed
            difference in unhealthy share, and the geometric mean of the
            per-seed ratios of lag p50 and p99 (B over A) with bootstrap 95 %
            intervals; each --metric F adds the geometric mean of the
            per-seed ratios of the row's field F (pairs where both are 0
            or missing drop out)
  track     how the seed's capacity reading (a pacer's estimate, or a known
            capacity) follows its true uplink: per cell, the median of its
            runs' reading over the uplink (p10, p50, p90), the seconds after
            each drop until it reads at most the new uplink and after each
            rise until it reads 80 % of it, the share of the uplink the seed's
            chunks used, the median viewer's first frame (seconds into the
            stream) and the honest viewers' lag p50 over their startup
            (the verdict's phases), and the row's off_drop and off_rise
            medians when it has them; --runs prints each run
  delays    how the node's queueing delay readings follow its true uplink
            queue, per cell: for the meter's pings (qd), vnic's path excess
            (qdv) and its queued bytes over the delivery rate (qdb), the
            median over runs of the reading's median error in ms, its false
            positives and false negatives (`Report::queue_delay_accuracy`),
            at seeds and at forwarders. A sim row has them; a testbed run's
            are its seed's lines against tc's tbf backlog (tc-0.log) over
            the stream, at the same threshold and window

The verdicts that `collapse`, `map`, `pair` and the rest judge runs by,
and the measures over time behind hack/sim-db's tables, are
hack/simlab/verdict.py's, which documents them; this module re-exports
their names.
"""

import argparse
import collections
import json
import math
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "hack"))

from simlab.verdict import (  # noqa: E402
    ABSORB_SECS, COLLAPSED_SECS, DELIVERY_MAX_MS, DELIVERY_SECS, DELIVERY_SHARE, DELIVERY_TARGET_MS, E2E_MAX_MS,
    E2E_TARGET_MS, LAG_HIGH_MS, LAG_HIGH_SECS, LAG_PERCENTILE, LAG_SLOPE, LAG_SLOPE_FLOOR_MS, LAG_SLOPE_SECS, OSC_ACF,
    OSC_MIN_PERIOD, OVERHEAD, PIPELINE_MS, RECOVER_SECS, STARTUP_SECS, STEP_SECS, TREES_SECS, TREES_SHARE,
    TREES_SMOOTH, VERDICT_VERSION, acf, collapsed, episodes, events, feasible, lags, measures, med, phase_lags, phases,
    presence, quantile, run_metrics, run_startup, runs_of, slope, startup, steps, timeline, verdict, verdict_version,
)

# The rows `reduce` keeps.
TYPES = "decisions,samples,DATA,REQUEST,STREAM_*"


def reduce(a):
    tool = a.bin or os.environ.get("PPSPP_CAPTURE") or REPO / "target/release/ppspp-capture"
    if not Path(tool).exists():
        sys.exit(f"no {tool}: cargo build --release -p strims-ppspp-capture")
    cmd = [str(tool), "swarm", "--labels", "--start"]
    if a.from_ is not None:
        cmd += ["--from", str(a.from_)]
    if a.until is not None:
        cmd += ["--until", str(a.until)]
    if not a.all:
        cmd += ["--types", TYPES]
    sys.exit(subprocess.call(cmd + [d for x in a.dumps for d in run_dumps(x)]))


def run_dumps(x):
    """`x`, or the one run under it: a STRIMS_SIM_CAPTURE root holds a run
    per `<test>/seed-<s>` (and its replay's), and one swarm view is one run's
    dumps, so a root with several runs names them and exits."""
    p = Path(x)
    if not p.is_dir() or any(p.glob("*.ppcap")):
        return [x]
    runs = sorted({str(f.parent) for f in p.glob("**/*.ppcap")})
    if len(runs) == 1:
        return runs
    if not runs:
        sys.exit(f"{x}: no .ppcap dumps under it")
    sys.exit(f"{x} holds {len(runs)} runs; reduce one of them:\n  " + "\n  ".join(runs))


def load(a):
    lo = a.from_ * 1000 if a.from_ is not None else float("-inf")
    hi = a.until * 1000 if a.until is not None else float("inf")
    rows = []
    with open(a.rows) as f:
        for line in f:
            r = json.loads(line)
            if lo <= r["t"] <= hi:
                rows.append(r)
    return rows


def ms(ns):
    """A duration field in ms; None when unknown (-1)."""
    return None if ns is None or ns < 0 else round(ns / 1e6, 1)


def every(rows, step_ms):
    """The rows at least `step_ms` apart."""
    last = float("-inf")
    for r in rows:
        if r["t"] - last >= step_ms:
            last = r["t"]
            yield r


def fields(r, skip=("t", "node", "peer", "dir", "frame", "swarm", "type")):
    return {k: v for k, v in r.items() if k not in skip}


def seed(a):
    rows = [r for r in load(a) if r["node"] == a.node]
    print("-- first wave, budget and window")
    for r in rows:
        if r["type"] in ("SeedFirstWave", "SeedBudget", "SeedWindow"):
            print(f"{r['t'] / 1000:7.3f} {r['type']:13s} {fields(r)}")
    print("-- the upload meter, and the transport's view beside it (a sample after the reading)")
    truth = [r for r in rows if r["type"] in ("SimConn", "Tcp")]
    j = 0
    for r in every([r for r in rows if r["type"] == "Upload"], a.step * 1000):
        while j < len(truth) and truth[j]["t"] < r["t"]:
            j += 1
        line = (
            f"{r['t'] / 1000:7.3f} meter rate {r['rate']:8d} busy {r['busy'] / 10:5.1f}% "
            f"capacity {r['capacity']:9d} hint {r['hint']:8d} queue delay {ms(r['queue_delay'])} ms"
        )
        if j < len(truth):
            s = truth[j]
            if s["type"] == "SimConn":
                line += f" | model uplink {s['uplink_rate']} queue {ms(s['uplink_queue'])} ms"
            else:
                line += f" | tcp srtt {ms(s['srtt'])} ms delivery {s['delivery_rate']}"
        print(line)
    print("-- admissions per second, by rule")
    per = collections.defaultdict(collections.Counter)
    for r in rows:
        if r["type"] in ("Admitted", "Rejected"):
            per[int(r["t"] // 1000)][f"{r['type']} {r['rule']}"] += 1
    for s in sorted(per):
        print(f"{s:5d}s {dict(per[s])}")
    print("-- holders")
    for r in rows:
        if r["type"] in ("HolderJoined", "HolderLeft", "HolderSilent", "HolderUnserved"):
            extra = f" why {r['why']} copies {r['copies']} rtt {ms(r['rtt'])} ms" if r["type"] == "HolderLeft" else ""
            print(f"{r['t'] / 1000:7.3f} {r['type']:14s} {r.get('peer')}{extra}")


# Per sample pass: the sums `queues` prints, by sample type.
SUMS = {
    "Queues": ("runner_pushes", "runner_answers", "link_pending", "write_held"),
    "SimConn": ("unsent", "in_flight", "pending"),
    "Tcp": ("unsent", "unacked"),
    "WebSocket": ("buffered",),
    "WebRtc": ("buffered",),
    "Path": ("in_flight", "queued", "written_since"),
}


def queues(a):
    passes = collections.defaultdict(collections.Counter)
    for r in load(a):
        if r["node"] != a.node or r["type"] not in SUMS:
            continue
        c = passes[r["t"]]
        for k in SUMS[r["type"]]:
            if r[k] >= 0:
                c[f"{r['type']}.{k}"] += r[k]
        if r["type"] == "Queues":
            c["blocked links"] += r["qos_blocked"] or r["link_blocked"]
            c["links"] += 1
        if r["type"] in ("SimConn", "Tcp"):
            c[f"{r['type']}.cwnd max"] = max(c[f"{r['type']}.cwnd max"], r["cwnd"])
        if r["type"] == "SimConn" and r["uplink_queue"] >= 0:
            c["SimConn.uplink_queue ms"] = round(r["uplink_queue"] / 1e6, 1)
    cols = sorted({k for c in passes.values() for k in c}, key=lambda k: (k.split(".")[0], k))
    print("t      " + "  ".join(cols))
    last = float("-inf")
    for t in sorted(passes):
        if t - last < a.step * 1000:
            continue
        last = t
        print(f"{t / 1000:6.1f} " + "  ".join(f"{passes[t][k]:>{len(k)}}" for k in cols))


def links(a):
    by = collections.defaultdict(dict)
    for r in load(a):
        if r["node"] == a.node and r.get("peer") in a.peers and r["type"] in SUMS:
            by[(r["t"], r["peer"])].update({f"{r['type']}.{k}": v for k, v in fields(r).items()})
    last = {}
    for (t, p) in sorted(by):
        if t - last.get(p, float("-inf")) < a.step * 1000:
            continue
        last[p] = t
        d = by[(t, p)]
        print(f"{t / 1000:6.1f} {p:>10s} " + " ".join(f"{k}={v}" for k, v in sorted(d.items())))


def paths(a):
    by = collections.defaultdict(dict)
    for r in load(a):
        if r["node"] != a.node or r.get("peer") is None or (a.peers and r["peer"] not in a.peers):
            continue
        if r["type"] in ("Path", "Queues", "SimConn", "Tcp"):
            by[(r["t"], r["peer"])][r["type"]] = r
    kb = lambda b: None if b is None or b < 0 else round(b / 1000, 1)
    pos = lambda v: v if v is not None and v >= 0 else None
    per = collections.defaultdict(lambda: collections.defaultdict(list))
    last = {}
    print("t      peer       rtt base excess (ms) | in_flight queued kB rate kB/s q_delay ms | held kB uplink_q ms")
    for (t, peer) in sorted(by):
        d = by[(t, peer)]
        p = d.get("Path")
        if p is None or p["samples"] == 0:
            continue
        rtt, base, recent = ms(p["rtt"]), ms(p["base"]), ms(p["recent_min"])
        excess = None if recent is None or base is None else max(0.0, round(recent - base, 1))
        rate = pos(p["rate"])
        qd = None if not rate or pos(p["queued"]) is None else round(1000 * p["queued"] / rate, 1)
        q, sc, tcp = d.get("Queues"), d.get("SimConn"), d.get("Tcp")
        held = None
        if q is not None:
            held = q["link_pending"] + q["write_held"]
            held += sum(max(0, sc[k]) for k in ("unsent", "pending")) if sc else 0
            held += max(0, tcp["unsent"]) if tcp else 0
        uplink = ms(sc["uplink_queue"]) if sc else None
        row = {"excess": excess, "queued": kb(p["queued"]), "q_delay": qd, "held": kb(held), "uplink": uplink}
        for k, v in row.items():
            if v is not None:
                per[peer][k].append(v)
        if t - last.get(peer, float("-inf")) < a.step * 1000:
            continue
        last[peer] = t
        f = lambda x: "-" if x is None else x
        print(
            f"{t / 1000:6.1f} {peer:>8s} {f(rtt):>6} {f(base):>5} {f(excess):>6} | {f(kb(p['in_flight'])):>9} {f(kb(p['queued'])):>9}"
            f" {f(kb(rate)):>9} {f(qd):>10} | {f(kb(held)):>7} {f(uplink):>11}"
        )
    print("-- medians per link: excess ms, q_delay ms, queued kB, held kB, uplink queue ms")
    for peer in sorted(per):
        m = {k: med(v) for k, v in per[peer].items()}
        print(f"{peer:>8s} {m.get('excess')} {m.get('q_delay')} {m.get('queued')} {m.get('held')} {m.get('uplink')}")


def prunes(a):
    rows = load(a)
    stale = [r for r in rows if r["type"] == "PushStale" and r["node"] == a.node]
    print(f"-- stale pushes at {a.node}: {len(stale)}, the first at {stale[0]['t'] / 1000 if stale else None} s")
    print("   per second", dict(sorted(collections.Counter(int(r["t"] // 1000) for r in stale).items())))
    waits = sorted(r["wait"] / 1e6 for r in stale[:200])
    if waits:
        print(f"   the first 200 waited {waits[0]:.0f} to {waits[-1]:.0f} ms, median {med(waits):.0f}")
    pushes = [r for r in rows if r["node"] == a.node and r["type"] == "DATA" and r["dir"] == "out" and r.get("ann") == "push"]
    per = collections.defaultdict(list)
    for r in pushes:
        per[int(r["t"] // 1000)].append(r.get("wait", 0) / 1e6)
    print(f"-- pushes {a.node} wrote per second, and their median wait in its queue (ms)")
    print("  ", {s: (len(v), round(med(v))) for s, v in sorted(per.items())})
    pruned = [r for r in rows if r["type"] == "Pruned"]
    print(f"-- prunes: {len(pruned)}, by rule {dict(collections.Counter(r['rule'] for r in pruned))}; the first {a.first}:")
    for r in pruned[: a.first]:
        print(
            f"  {r['t'] / 1000:7.3f} {r['node']} pruned {r['peer']} s{r['stream']} by {r['rule']}: "
            f"{r['silent_misses']} misses since a hit {ms(r['since_hit'])} ms ago, miss rate {r['miss_rate'] / 1e6:.2f}, "
            f"expected {r['expected']}, pushes {r['pushes']}, age {ms(r['age'])} ms; push delay mean {ms(r['lag_mean'])} "
            f"dev {ms(r['lag_dev'])} peak {ms(r['lag_peak'])} ms, deadline {ms(r['lag_deadline'])} ms"
        )
    if not pruned:
        return
    p = pruned[0]
    misses = [r for r in rows if r["type"] == "PushMiss" and r["node"] == p["node"] and r.get("stream") == p["stream"] and r["t"] <= p["t"]]
    print(f"-- {p['node']}'s misses on s{p['stream']} up to its first prune: {len(misses)}; the last 6:")
    for r in misses[-6:]:
        print(
            f"  {r['t'] / 1000:7.3f} bin {r['bin']} first seen {r['first_seen'] / 1000:.3f} deadline "
            f"{r['deadline'] / 1000:.3f} ({r['deadline'] - r['first_seen']:.0f} ms) counted {r['counted']} "
            f"parent {r.get('peer')}; its push delay now mean {ms(r['lag_mean'])} deadline {ms(r['lag_deadline'])} ms"
        )
    got = [
        r for r in rows
        if r["type"] == "DATA" and r["dir"] == "in" and r["node"] == p["node"] and r.get("peer") == p["peer"]
        and r.get("stream") == p["stream"] and r.get("ann") in ("hit", "late") and r["t"] <= p["t"]
    ]
    print(f"   {p['peer']}'s pushes of s{p['stream']} that arrived before it (t, as, transit ms):",
          [(r["t"] / 1000, r["ann"], ms(r.get("transit"))) for r in got[-6:]])


def pulls(a):
    rows = [r for r in load(a) if r["node"] != a.source]
    where = lambda r: "source" if r["peer"] == a.source else "viewers"
    req = collections.defaultdict(collections.Counter)
    for r in rows:
        if r["type"] == "REQUEST" and r["dir"] == "out":
            req[int(r["t"] // 1000)][where(r)] += 1
    print(f"-- chunks requested per second, to {a.source} / to viewers (share to {a.source})")
    for s in sorted(req):
        c = req[s]
        print(f"  {s:5d}s {c['source']:6d} / {c['viewers']:6d}  ({c['source'] / max(c['source'] + c['viewers'], 1):.0%})")
    print("-- request rounds per 2 s, medians: window cap, bytes in flight, chunks requested, candidates, room")
    rr = collections.defaultdict(lambda: collections.defaultdict(list))
    for r in rows:
        if r["type"] == "RequestRound":
            d = rr[(int(r["t"] // 2000) * 2, where(r))]
            for f in ("cap", "in_flight", "requested", "candidates", "room"):
                d[f].append(r[f])
    for (b, k), d in sorted(rr.items()):
        print(
            f"  {b:3d}-{b + 2}s to {k:7s} rounds {len(d['cap']):6d} cap {med(d['cap']):7d} in flight "
            f"{med(d['in_flight']):7d} requested {med(d['requested']):3d} candidates {med(d['candidates']):4d} room {med(d['room'])}"
        )
    to = collections.defaultdict(collections.Counter)
    for r in rows:
        if r["type"] == "RequestTimeout":
            to[int(r["t"] // 1000)][where(r)] += 1
    print(f"-- request timeouts per second, to {a.source} / to viewers:",
          {s: (c["source"], c["viewers"]) for s, c in sorted(to.items())})


def node(a):
    rows = [r for r in load(a) if r["node"] == a.node]
    per = collections.defaultdict(collections.Counter)
    for r in rows:
        k = None
        if r["type"] in ("Admitted", "Rejected"):
            k = f"{r['type']} {r['rule']}"
        elif r["type"] == "DATA":
            # Unmarked DATA in answers this node's request.
            k = f"DATA {r['dir']} {r.get('ann') or ('answer' if r['dir'] == 'in' else 'unmarked')}"
        elif r["dir"] == "tap" and r["type"] not in ("Upload", "RequestRound", "DataSent", "PushHit"):
            k = r["type"]
        if k:
            per[int(r["t"] // 1000)][k] += 1
    for s in sorted(per):
        print(f"{s:5d}s {dict(per[s])}")
    print("-- the first admissions, with their numbers")
    for r in [r for r in rows if r["type"] in ("Admitted", "Rejected")][: a.first]:
        print(f"  {r['t'] / 1000:7.3f} {r['type']} {r['peer']} s{r['stream']} {fields(r, ('t', 'node', 'peer', 'dir', 'frame', 'swarm', 'type', 'stream'))}")
    print("-- the upload meter")
    for r in every([r for r in rows if r["type"] == "Upload"], a.step * 1000):
        print(f"  {r['t'] / 1000:7.3f} {fields(r)}")


# Env keys that only number a run, not a cell.
SEED_KEYS = {"STRIMS_SIM_SEED", "SIM_SEED", "seed", "repeat"}
PREFIXES = ("STRIMS_SIM_CAL_", "STRIMS_SIM_", "SIM_")


def short(k):
    for p in PREFIXES:
        if k.startswith(p):
            return k[len(p):]
    return k


def settings(path):
    """A testbed run's `settings` (run.sh's variables that were set)."""
    env = {}
    if path.exists():
        for line in path.read_text().splitlines():
            k, _, v = line.partition("=")
            env[k] = v
    return env


def node_arg(args, flag, default):
    a = args.split()
    return int(a[a.index(flag) + 1]) if flag in a else default


def testbed_series(d, env):
    """A testbed run dir's series, in `Report::series_data`'s fields."""
    env = {**settings(d / "settings"), **env}
    viewers = int(env.get("VIEWERS", 3))
    args = env.get("NODE_ARGS", "")
    rate = node_arg(args, "--rate", 256 * 1024)
    chunk = node_arg(args, "--chunk", 1024)
    secs = int(env.get("SECS", 180))
    seed_up = float(env.get("SEED_UP_MBIT", 10)) * 125000
    steps = [[float(a), float(m) * 125000] for a, m in (p.split(":") for p in env.get("SEED_UP_STEPS", "").split())]
    nodes = []
    for i in range(viewers + 1):
        f = d / f"node-{i}.jsonl"
        lines = [json.loads(x) for x in f.read_text().splitlines() if x.startswith("{")] if f.exists() else []
        # A line covers the second before it; one without transfers comes
        # before the node joined the swarm.
        nodes.append({round(r["rel_ms"] / 1000) - 1: r for r in lines if r.get("transfers")})
    n = secs + 5
    ks = range(n)
    s = {
        "rate": rate, "chunk": chunk, "frame": 1024, "secs": secs,
        "seed_up": [[0, seed_up]] + steps,
        "viewer_up": viewers * float(env.get("VIEWER_UP_MBIT", 20)) * 125000,
        "lag": [[v[k]["lag_p99_ms"] if k in v and v[k]["frames"] else -1 for k in ks] for v in nodes[1:]],
        "frames": [[v[k]["frames"] if k in v else 0 for k in ks] for v in nodes[1:]],
        "present": [sum(k in v for v in nodes[1:]) for k in ks],
    }

    def delta(v, k, f):
        if k not in v or k - 1 not in v:
            return 0
        a, b = v[k]["transfers"][f], v[k - 1]["transfers"][f]
        return a - b if a >= b else a

    for f in ("hits", "misses", "pruned"):
        s[f] = [sum(delta(v, k, f) for v in nodes[1:]) for k in ks]
    parents = lambda k: med([v[k]["transfers"]["parents"] for v in nodes[1:] if k in v])
    s["parents"] = [-1 if parents(k) is None else parents(k) for k in ks]
    seed = nodes[0]
    s["capacity"] = [seed[k]["capacity"] if k in seed else -1 for k in ks]
    s["busy"] = [seed[k]["busy_permille"] if k in seed else -1 for k in ks]
    s["copies"] = [seed[k]["seed_copies"] if k in seed else -1 for k in ks]
    s["sent"] = [delta(seed, k, "chunks_sent") for k in ks]
    s["pushed"] = [delta(seed, k, "pushed_chunks") for k in ks]
    up = lambda k: [u for t, u in s["seed_up"] if t <= k + 0.5][-1]
    s["avail"] = [up(k) for k in ks]
    return s


def query():
    """simlab.query: the farm's tables, in hack/sim-db's cache (the duckdb
    module's: VERDICT_COMMANDS run under the engine)."""
    from simlab import query

    return query


def index_of(p):
    """A test matrix's index.jsonl, if `p` is one or its dir."""
    index = p / "index.jsonl" if p.is_dir() else p
    return index if index.name == "index.jsonl" and index.exists() else None


def index_rows(index):
    """(line, row, series) per job of a farm matrix's index: its row and
    series from its tables, pulled into hack/sim-db's cache; a job whose
    tables are gone (S3 keeps them a week) is left out."""
    recs = [r for r in map(json.loads, filter(str.strip, index.read_text().splitlines())) if r.get("src")]
    gone = set(query().pull_sources({r["src"] for r in recs}))
    got = query().rows([(r["src"], r["key"]) for r in recs if r["src"] not in gone], series=True)
    for r in recs:
        row = got.get((r["src"], r["key"]))
        if row is not None:
            row = dict(row)  # a verify copy's line shares its original's
            yield r, row, row.pop("_series")


def line_env(r):
    return {**r.get("env", {}), "arm": r.get("arm", ""), "commit": r.get("commit", "")[:8], "test": r.get("test", "")}


def runs(paths):
    """(label, env, row, series) per run in `paths` (see the module docs)."""
    for p in map(Path, paths):
        if p.is_dir() and (p / "node-0.jsonl").exists():
            yield p.name, {}, {}, testbed_series(p, {})
            continue
        if (index := index_of(p)) is not None:
            for r, row, s in index_rows(index):
                yield r["id"], line_env(r), row, s
            continue
        rows = p / "rows.jsonl" if p.is_dir() else p
        with open(rows) as f:
            for line in f:
                # A test's output may share the harness's "test ... " line.
                at = line.find("ROW {")
                line = line[at + 4:] if at >= 0 else line.strip()
                if not line.startswith("{"):
                    continue
                try:
                    r = json.loads(line)
                except ValueError:
                    continue  # a torn append's
                if "row" in r:
                    env = {
                        **r.get("env", {}),
                        "arm": r.get("arm", ""),
                        "commit": r.get("commit", "")[:8],
                        "test": r.get("test", ""),
                    }
                    if r.get("kind") == "testbed":
                        env["repeat"] = r.get("repeat")
                        yield r["id"], env, r["row"], testbed_series(rows.parent / r["run_dir"], r.get("env", {}))
                    elif "series" in r["row"]:
                        yield r["id"], env, r["row"], r["row"]["series"]
                elif "series" in r:
                    yield f"{r.get('test', r.get('scenario'))} seed {r.get('seed')}", {"seed": r.get("seed")}, r, r["series"]


# `delays`: the readings, and how `Report::queue_delay_accuracy` judges
# them (crates/strims-sim/src/measures.rs).
# (row prefix, node line field, scored against the wait at the moment rather
# than the standing queue), as strims-sim's Report::row scores them.
DELAY_READINGS = (
    ("qd", "queue_delay_ms", False),
    ("qdv", "path_delay_ms", False),
    ("qdb", "path_queued_ms", False),
    ("qdbi", "path_queued_ms", True),
    ("qdl", "path_latest_ms", True),
)
STANDING_MS = 3000
SATURATED_STANDING_MS = 5
QUEUE_DELAY_THRESHOLD_MS = 80


def tc_queue(path):
    """(wall ms, ms to drain the tbf backlog at its rate) per line of a
    testbed tc log (run.sh's tc-0.log)."""
    import re

    unit = {"": 1, "K": 1024, "M": 1024 * 1024, "G": 1024 ** 3}
    rate_unit = {"": 1, "K": 1e3, "M": 1e6, "G": 1e9}
    out = []
    for line in path.read_text().splitlines():
        t, _, rest = line.partition(" ")
        tbf = rest.partition("qdisc tbf")[2]
        b = re.search(r"backlog ([\d.]+)([KMG]?)b", tbf)
        r = re.search(r"rate ([\d.]+)([KMG]?)bit", tbf)
        if not (t.isdigit() and b and r):
            continue
        backlog = float(b[1]) * unit[b[2]]
        rate = float(r[1]) * rate_unit[r[2]] / 8
        out.append((int(t), 1000 * backlog / rate))
    return out


def testbed_delays(d):
    """A testbed run's `<reading>_{err,fp,fn}_seeds*` fields: its seed's
    lines against tc's standing queue, or its wait at the moment, over the
    stream."""
    env = settings(d / "settings")
    secs = int(env.get("SECS", 180))
    queue = tc_queue(d / "tc-0.log") if (d / "tc-0.log").exists() else []
    lines = [json.loads(x) for x in (d / "node-0.jsonl").read_text().splitlines() if x.startswith("{")]
    out = {}
    for name, field, instant in DELAY_READINGS:
        errs, neg, pos_ = [], [0, 0], [0, 0]
        for ln in lines:
            v = ln.get(field)
            if v is None or not 0 <= ln["rel_ms"] <= secs * 1000:
                continue
            window = [w for t, w in queue if ln["t_ms"] - STANDING_MS <= t <= ln["t_ms"]]
            if not window:
                continue
            wait = window[-1] if instant else min(window)
            errs.append(abs(v - wait))
            if wait < (QUEUE_DELAY_THRESHOLD_MS if instant else SATURATED_STANDING_MS):
                neg[1] += 1
                neg[0] += v >= QUEUE_DELAY_THRESHOLD_MS
            else:
                pos_[1] += 1
                pos_[0] += v < QUEUE_DELAY_THRESHOLD_MS
        if errs:
            out[f"{name}_err_seeds_ms"] = med(errs)
            out[f"{name}_fp_seeds"] = neg[0] / neg[1] if neg[1] else 0.0
            out[f"{name}_fn_seeds"] = pos_[0] / pos_[1] if pos_[1] else 0.0
    return out


def delay_rows(paths):
    """(env, row) per run in `paths`: sim rows as they are, testbed runs
    with the fields `testbed_delays` gives."""
    for p in map(Path, paths):
        if p.is_dir() and (p / "node-0.jsonl").exists():
            yield settings(p / "settings"), testbed_delays(p)
            continue
        if (index := index_of(p)) is not None:
            for r, row, _ in index_rows(index):
                yield line_env(r), row
            continue
        rows = p / "rows.jsonl" if p.is_dir() else p
        with open(rows) as f:
            for line in f:
                at = line.find("ROW {")
                line = line[at + 4:] if at >= 0 else line.strip()
                if not line.startswith("{"):
                    continue
                try:
                    r = json.loads(line)
                except ValueError:
                    continue  # a torn append's
                if "row" in r:
                    env = {
                        **r.get("env", {}),
                        "arm": r.get("arm", ""),
                        "commit": r.get("commit", "")[:8],
                        "test": r.get("test", ""),
                    }
                    if r.get("kind") == "testbed":
                        env["repeat"] = r.get("repeat")
                        yield env, testbed_delays(rows.parent / r["run_dir"])
                    else:
                        yield env, r["row"]
                else:
                    yield {"seed": r.get("seed"), "test": r.get("test", "")}, r


def delays(a):
    done = [(env, row) for env, row in delay_rows(a.runs) if matches(env, row, a.where)]
    keys = {k for env, _ in done for k in env}
    if a.group:
        group = [k for g in a.group.split(",") for k in sorted(keys) if short(k) == short(g)]
    else:
        group = sorted(k for k in keys if k not in SEED_KEYS and len({str(e.get(k)) for e, _ in done}) > 1)
    cells = collections.defaultdict(list)
    for env, row in done:
        cells[tuple(str(env.get(k, "")) for k in group)].append(row)
    num = lambda x: (0, float(x)) if x.replace(".", "", 1).isdigit() else (1, x)
    f = lambda x, fmt: "-" if x is None else format(x, fmt)
    cols = " | ".join(f"{n} err fp fn" for n, _, _ in DELAY_READINGS)
    print(" ".join(short(k) for k in group) + f" | role n | {cols}")
    for cell in sorted(cells, key=lambda c: [num(x) for x in c]):
        rows = cells[cell]
        for role in ("seeds", "forwarders"):
            parts = []
            for name, _, _ in DELAY_READINGS:
                v = lambda k: med([r[k] for r in rows if r.get(k) is not None])
                parts.append(
                    f"{f(v(f'{name}_err_{role}_ms'), '.0f')} {f(v(f'{name}_fp_{role}'), '.2f')} {f(v(f'{name}_fn_{role}'), '.2f')}"
                )
            n = sum(1 for r in rows if r.get(f"qd_err_{role}_ms") is not None or r.get(f"qdv_err_{role}_ms") is not None)
            if n:
                print(f"{' '.join(cell)} | {role} {n} | " + " | ".join(parts))


def matches(env, row, where):
    for w in where or ():
        k, _, v = w.partition("=")
        have = {short(x): str(y) for x, y in {**row, **env}.items()}
        if have.get(short(k), "") != v:
            return False
    return True


def series(a):
    for label, env, row, s in runs(a.runs):
        if not matches(env, row, a.where):
            continue
        n, smooth, deliv, lag = measures(s)
        vl = lags(s)
        print(f"== {label} {' '.join(f'{short(k)}={v}' for k, v in sorted(env.items()) if v not in (None, ''))}")
        # Overhead written (not chunk data, the transports' framing aside):
        # the seed's, and the mean viewer's.
        ovh = "seed_ovh" in s
        print("    s viewers ontime deliv   lag_ms pruned parents  seed: cap_kB avail_kB busy copies sent pushed%"
              + ("  ovh_kB: seed viewer" if ovh else ""))
        for k in range(0, len(s["present"]), a.every):
            f = lambda x, fmt: format(x, fmt) if x is not None else "-"
            sent = s["sent"][k]
            tail = ""
            if ovh:
                per = s["viewer_ovh"][k] / 1000 / s["present"][k] if s["present"][k] else None
                tail = f"  {s['seed_ovh'][k] / 1000:12.1f} {f(per, '6.1f')}"
            print(
                f"{k:5d} {s['present'][k]:7d} {f(smooth[k] if k < n else None, '6.2f')} {f(deliv[k] if k < n else None, '5.2f')}"
                f" {f(quantile([v[k] for v in vl if v[k] is not None], LAG_PERCENTILE), '8d')} {s['pruned'][k]:6d} {s['parents'][k]:7d}"
                f"  {s['capacity'][k] // 1000 if s['capacity'][k] >= 0 else '-':>12} {s['avail'][k] // 1000 if s['avail'][k] >= 0 else '-':>8}"
                f" {s['busy'][k] / 10 if s['busy'][k] >= 0 else '-':>5} {s['copies'][k]:6d} {sent:4d} {100 * s['pushed'][k] // sent if sent else '-':>7}"
                + tail
            )


def collapse(a):
    print("run  trees delivery latency recovered absorbing feasible lag_p50 lag_max")
    for label, env, row, s in runs(a.runs):
        if not matches(env, row, a.where):
            continue
        v = verdict(s)
        f = lambda x: "-" if x is None else x
        print(
            f"{label} {' '.join(f'{short(k)}={x}' for k, x in sorted(env.items()) if x not in (None, '') and k != 'arm' or (k == 'arm' and x))}: "
            f"{f(v['trees'])} {f(v['delivery'])} {f(v['latency'])} {f(v['recovered'])} {v['absorbing']} {v['feasible']} {f(v['lag_p50'])} {f(v['lag_max'])}"
        )


def map_(a):
    done = [(env, verdict(s)) for _, env, row, s in runs(a.runs) if matches(env, row, a.where)]
    keys = {k for env, _ in done for k in env}
    if a.group:
        group = [k for g in a.group.split(",") for k in sorted(keys) if short(k) == short(g)]
    else:
        group = sorted(k for k in keys if k not in SEED_KEYS and len({str(e.get(k)) for e, _ in done}) > 1)
    cells = collections.defaultdict(list)
    for env, v in done:
        cells[tuple(str(env.get(k, "")) for k in group)].append(v)
    pct = lambda vs, f: f"{100 * sum(1 for v in vs if f(v)) // len(vs):3d}%"
    onset = lambda vs, k: med([v[k] for v in vs if v[k] is not None])
    num = lambda x: (0, float(x)) if x.replace(".", "", 1).isdigit() else (1, x)
    print(" ".join(short(k) for k in group) + " | n feasible | trees delivery latency absorbing | unhealthy | onsets: trees delivery latency | lag_p50")
    for cell in sorted(cells, key=lambda c: [num(x) for x in c]):
        vs = cells[cell]
        fe = "/".join(sorted({str(v["feasible"]) for v in vs}))
        print(
            f"{' '.join(cell)} | {len(vs)} {fe} | {pct(vs, lambda v: v['trees'] is not None)} {pct(vs, lambda v: v['delivery'] is not None)}"
            f" {pct(vs, lambda v: v['latency'] is not None)} {pct(vs, lambda v: v['absorbing'])} |"
            f" {100 * (med([v['unhealthy'] for v in vs if v['unhealthy'] is not None]) or 0):3.0f}% |"
            f" {onset(vs, 'trees')} {onset(vs, 'delivery')} {onset(vs, 'latency')} | {med([v['lag_p50'] for v in vs if v['lag_p50'] is not None])}"
        )


BOOTSTRAP = 2000


def selector(spec):
    """A --a/--b selector: K=V pairs, comma-separated."""
    return [w for w in spec.split(",") if w]


def boot_ci(v, rounds=BOOTSTRAP):
    """The mean of `v` and a bootstrap 95 % interval, from a fixed seed."""
    import random

    if not v:
        return None, None, None
    rng = random.Random(0x5EED)
    means = sorted(sum(rng.choice(v) for _ in v) / len(v) for _ in range(rounds))
    return sum(v) / len(v), means[int(0.025 * rounds)], means[int(0.975 * rounds) - 1]


def pair(a):
    import math

    sel_a = selector(a.a)
    sels_b = [selector(b) for b in a.b]
    arm_keys = {short(w.partition("=")[0]) for sel in [sel_a, *sels_b] for w in sel}
    arm_keys |= {short(k) for k in (a.ignore or "").split(",") if k}
    arm_keys |= {"arm", "commit"}
    seed_keys = {short(k) for k in SEED_KEYS}
    found = []
    for _, env, row, s in runs(a.runs):
        if not matches(env, row, a.where):
            continue
        which = [i for i, sel in enumerate([sel_a, *sels_b]) if matches(env, row, sel)]
        if not which:
            continue
        found.append((env, row, s, which))
    cell_keys = sorted({short(k) for env, *_ in found for k in env} - arm_keys - seed_keys)

    def key(env):
        e = {short(k): str(v) for k, v in env.items()}
        seed = next((e[k] for k in seed_keys if k in e), "")
        return tuple(e.get(k, "") for k in cell_keys), seed

    arms = [collections.defaultdict(dict) for _ in range(1 + len(sels_b))]
    for env, row, s, which in found:
        cell, seed = key(env)
        v = verdict(s)
        for i in which:
            arms[i][cell][seed] = (v, row)
    shown = [k for k in cell_keys if len({c[cell_keys.index(k)] for arm in arms for c in arm}) > 1]
    num = lambda x: (0, float(x)) if x.replace(".", "", 1).isdigit() else (1, x)
    ratio = lambda xs: "-" if xs[0] is None else f"{math.exp(xs[0]):.2f} [{math.exp(xs[1]):.2f},{math.exp(xs[2]):.2f}]"
    print(f"A = {a.a}")
    for i, b in enumerate(a.b):
        print(f"B{i + 1} = {b}")
    metrics = a.metric or []
    extra = "".join(f" | {m} B/A" for m in metrics)
    print(" ".join(shown) + " | B | pairs | unhealthy A B | absorbing A B | d unhealthy | lag_p50 B/A | lag_p99 B/A" + extra)
    for cell in sorted(arms[0], key=lambda c: [num(x) for x in c]):
        label = " ".join(cell[cell_keys.index(k)] or "-" for k in shown)
        for i in range(1, len(arms)):
            seeds = sorted(set(arms[0][cell]) & set(arms[i].get(cell, {})), key=lambda x: num(x))
            if not seeds:
                continue
            pa = [arms[0][cell][x] for x in seeds]
            pb = [arms[i][cell][x] for x in seeds]
            un = lambda ps: med([v["unhealthy"] for v, _ in ps if v["unhealthy"] is not None]) or 0
            ab = lambda ps: sum(1 for v, _ in ps if v["absorbing"])
            d = med([(vb["unhealthy"] or 0) - (va["unhealthy"] or 0) for (va, _), (vb, _) in zip(pa, pb)])
            logs = lambda f: [
                math.log(rb[f] / ra[f]) for (_, ra), (_, rb) in zip(pa, pb) if ra.get(f) and rb.get(f)
            ]
            print(
                f"{label} | B{i} | {len(seeds)} | {100 * un(pa):3.0f}% {100 * un(pb):3.0f}% | {ab(pa)} {ab(pb)} |"
                f" {100 * (d or 0):+4.0f} | {ratio(boot_ci(logs('lag_p50_ms')))} | {ratio(boot_ci(logs('lag_p99_ms')))}"
                + "".join(f" | {ratio(boot_ci(logs(m)))} ({len(logs(m))})" for m in metrics)
            )


TRACK_RISE = 0.8


def track_run(s, row):
    """One run's tracking measures (see `track`)."""
    n = min(s["secs"], len(s["capacity"]))
    up = lambda k: [u for t, u in s["seed_up"] if t <= k + 0.5][-1]
    ratios = [s["capacity"][k] / up(k) for k in range(n) if s["capacity"][k] > 0 and up(k) > 0]
    falls, climbs = [], []
    for (t, u), (_, before) in zip(s["seed_up"][1:], s["seed_up"]):
        t = int(round(t))
        later = [k for k in range(t, n) if s["capacity"][k] > 0]
        if u < before:
            falls.append(next((k - t for k in later if s["capacity"][k] <= u), None))
        elif u > before:
            climbs.append(next((k - t for k in later if s["capacity"][k] >= TRACK_RISE * u), None))
    used = med([s["sent"][k] * s["chunk"] / up(k) for k in range(n) if up(k) > 0])
    firsts = [next((k for k, f in enumerate(frames) if f), None) for frames in s["frames"]]
    off = lambda f: row.get(f, [None])[0] if isinstance(row.get(f), list) else None
    return {
        "ratio": (quantile(ratios, 10), quantile(ratios, 50), quantile(ratios, 90)),
        "fall": falls, "climb": climbs, "used": used,
        "first": med([x for x in firsts if x is not None]),
        "startup": quantile(phase_lags(s).get("startup", []), 50),
        "off_drop": off("off_drop"), "off_rise": off("off_rise"),
    }


def track(a):
    done = [(label, env, track_run(s, row)) for label, env, row, s in runs(a.runs) if matches(env, row, a.where)]
    keys = {k for _, env, _ in done for k in env}
    if a.group:
        group = [k for g in a.group.split(",") for k in sorted(keys) if short(k) == short(g)]
    else:
        group = sorted(k for k in keys if k not in SEED_KEYS and len({str(e.get(k)) for _, e, _ in done}) > 1)
    f = lambda x, fmt=".2f": "-" if x is None else format(x, fmt)
    lst = lambda xs: ",".join("-" if x is None else str(x) for x in xs) or "-"
    if a.runs_:
        print("run | reading/uplink p10 p50 p90 | fall_s | climb_s | used | first_s startup_ms | off_drop off_rise")
        for label, env, t in done:
            cell = " ".join(f"{short(k)}={env.get(k)}" for k in group)
            r = t["ratio"]
            print(
                f"{label} {cell} | {f(r[0])} {f(r[1])} {f(r[2])} | {lst(t['fall'])} | {lst(t['climb'])} |"
                f" {f(t['used'])} | {f(t['first'], 'd')} {f(t['startup'], '.0f')} | {f(t['off_drop'])} {f(t['off_rise'])}"
            )
        return
    cells = collections.defaultdict(list)
    for _, env, t in done:
        cells[tuple(str(env.get(k, "")) for k in group)].append(t)
    num = lambda x: (0, float(x)) if x.replace(".", "", 1).isdigit() else (1, x)
    # A drop or rise the reading never followed counts as the stream's length.
    never = lambda ts, k: med([x if x is not None else 10**6 for t in ts for x in t[k]])
    print(" ".join(short(k) for k in group) + " | n | reading/uplink p10 p50 p90 | fall_s | climb_s | used | first_s startup_ms | off_drop off_rise")
    for cell in sorted(cells, key=lambda c: [num(x) for x in c]):
        ts = cells[cell]
        r = [med([t["ratio"][i] for t in ts if t["ratio"][i] is not None]) for i in range(3)]
        fall, climb = never(ts, "fall"), never(ts, "climb")
        nv = lambda x: "never" if x == 10**6 else f(x, "d") if isinstance(x, int) else f(x, ".1f")
        print(
            f"{' '.join(x or '-' for x in cell)} | {len(ts)} | {f(r[0])} {f(r[1])} {f(r[2])} | {nv(fall)} | {nv(climb)} |"
            f" {f(med([t['used'] for t in ts if t['used'] is not None]))} | {f(med([t['first'] for t in ts if t['first'] is not None]), '.1f')}"
            f" {f(med([t['startup'] for t in ts if t['startup'] is not None]), '.0f')} |"
            f" {f(med([t['off_drop'] for t in ts if t['off_drop'] is not None]))} {f(med([t['off_rise'] for t in ts if t['off_rise'] is not None]))}"
        )


# A copy sent within this long of a chunk's first copy is of a fresh chunk.
FRESH_MS = 2000


def answers(a):
    rows = [r for r in load(a) if r["node"] == a.node and r["type"] == "DATA" and r["dir"] == "out"]
    first = {}
    per = collections.defaultdict(lambda: collections.defaultdict(list))
    for r in rows:
        b = (r["swarm"], r["bin"])
        f = first.setdefault(b, r["t"])
        d = per[int(r["t"] // 1000)]
        d["new"].append(f == r["t"])
        d[r.get("ann") or "answer"].append(r.get("wait", 0) / 1e6)
        if r.get("ann") != "push" and r["t"] - f <= FRESH_MS:
            d["fresh"].append(b)
    print(f"-- {a.node}'s DATA per second: chunks new to it, pushes, answers, answers of fresh chunks (sent within"
          f" {FRESH_MS} ms of a chunk's first copy) per fresh chunk answered, median waits in its queue (ms)")
    for k in sorted(per):
        d = per[k]
        fresh = collections.Counter(d["fresh"])
        print(
            f"  {k:5d}s new {sum(d['new']):5d} push {len(d['push']):5d} answer {len(d['answer']):5d} fresh answers"
            f" {len(d['fresh']):5d} per chunk {len(d['fresh']) / max(len(fresh), 1):4.1f} (max {max(fresh.values(), default=0)})"
            f" wait push {med(d['push']) or 0:6.0f} answer {med(d['answer']) or 0:6.0f}"
        )


def gate(a):
    """Per second at `--node`: the answer pace's rate and the queueing delay
    it read (as of its latest tick), the requests that arrived, and the
    pushes and answers sent."""
    per = collections.defaultdict(collections.Counter)
    rate = {}
    for r in load(a):
        if r["node"] != a.node:
            continue
        k = int(r["t"] // 1000)
        d = per[k]
        t = r["type"]
        if t == "AnswerRate":
            rate[k] = (r["rate"], ms(r["reading"]))
        elif t == "REQUEST" and r.get("dir") == "in":
            d["requests"] += 1
        elif t == "DATA" and r.get("dir") == "out":
            d["push" if r.get("ann") == "push" else "answer"] += 1
    print(f"-- {a.node}'s answer pace per second: rate (kB/s) and delay read (ms), requests in, pushes and"
          " answers out")
    last = (None, None)
    for k in sorted(per):
        last = rate.get(k, last)
        d = per[k]
        r = "-" if last[0] is None else f"{last[0] // 1000:6d}"
        print(
            f"  {k:5d}s rate {r:>6} read {'-' if last[1] is None else last[1]:>6} req {d['requests']:5d}"
            f" push {d['push']:5d} answer {d['answer']:5d}"
        )


def holders(a):
    rows = load(a)
    spans = collections.defaultdict(list)
    for r in rows:
        if r["node"] == a.node and r["type"] in ("HolderJoined", "HolderLeft") and "peer" in r:
            if r["type"] == "HolderJoined":
                spans[r["peer"]].append([r["t"], None, None])
            elif spans[r["peer"]]:
                spans[r["peer"]][-1][1:] = [r["t"], r["why"]]
    stream = med([r["stream_rate"] for r in rows if r["type"] == "SeedBudget"])
    per = collections.defaultdict(lambda: collections.defaultdict(list))
    for r in rows:
        k = int(r["t"] // 1000)
        if r["node"] == a.node and r.get("peer") in spans:
            d = per[(r["peer"], k)]
            if r["type"] == "DATA" and r["dir"] == "out" and r.get("ann") == "push":
                d["pushed"].append(r["len"])
            elif r["type"] == "Queues":
                d["queued"].append(r["runner_pushes"])
            elif r["type"] in ("SimConn", "Tcp"):
                d["cwnd"].append(r["cwnd"])
                d["flight"].append(r["in_flight"] if r["type"] == "SimConn" else r["unacked"])
                d["pending"].append(max(r.get("pending", 0), 0) + max(r["unsent"], 0))
            elif r["type"] == "Path" and r["samples"] > 0:
                for k in ("rtt", "base", "recent_min"):
                    if ms(r[k]) is not None:
                        d[k].append(ms(r[k]))
                if r["queued"] >= 0:
                    d["path_queued"].append(r["queued"])
        elif r["node"] in spans:
            d = per[(r["node"], k)]
            if r["type"] == "DATA" and r["dir"] == "out" and r.get("ann") == "push":
                d["relayed"].append(r["len"])
            elif r["type"] in ("PushStale", "PushSuppressed", "PushCancelled", "DataDropped"):
                d["dropped"].append(r["type"])
        if r["type"] == "Pruned" and r.get("peer") in spans:
            per[(r["peer"], k)]["pruned by"].append(f"{r['node']}:{r['rule']}")
    print(f"-- {a.node}'s holders; a copy is its streams times {stream} B/s a stream (the seed's budget rate)")
    for h in sorted((h for h in spans if spans[h]), key=lambda h: spans[h][0][0]):
        if a.holder and h != a.holder:
            continue
        for t0, t1, why in spans[h]:
            print(f"== {h} held {t0 / 1000:.3f}-{'end' if t1 is None else f'{t1 / 1000:.3f}'} s{'' if why is None else f' (left: {why})'}")
            last = (int(t1 // 1000) if t1 is not None else max(k for p, k in per if p == h)) + 3
            for k in range(int(t0 // 1000), last + 1):
                d = per.get((h, k))
                if not d:
                    continue
                path = " ".join(f"{med(d[k]) if d[k] else '-':>5}" for k in ("rtt", "base", "recent_min"))
                pq = med(d["path_queued"]) // 1000 if d["path_queued"] else "-"
                print(
                    f"  {k:5d}s pushed {sum(d['pushed']) // 1000:5d} kB queued {med(d['queued']) if d['queued'] else '-':>5}"
                    f" cwnd {med(d['cwnd']) or '-':>6} in flight {med(d['flight']) or '-':>7} waiting {med(d['pending']) or '-':>8}"
                    f" | path rtt base recent {path} ms queued {pq:>5} kB"
                    f" | relayed {sum(d['relayed']) // 1000:5d} kB dropped {dict(collections.Counter(d['dropped']))}"
                    f" pruned by {sorted(collections.Counter(d['pruned by']).items())}"
                )
    joined = sum(len(v) for v in spans.values())
    left = collections.Counter(why for v in spans.values() for _, t1, why in v if t1 is not None)
    print(f"-- {joined} holder spans over {len(spans)} peers; left: {dict(left)}")


# Runs part where one's count is this many times the other's.
DIVERGE_RATIO = 2


def per_second(rows, source):
    per = collections.defaultdict(collections.Counter)
    for r in rows:
        c = per[int(r["t"] // 1000)]
        t, n = r["type"], r["node"]
        if n == source:
            if t == "DATA" and r["dir"] == "out":
                c["pushes" if r.get("ann") == "push" else "answers"] += 1
            elif t == "SeedWindow":
                c["copies"] = r["copies"]
            elif t == "Upload":
                c["busy%"] = r["busy"] // 10
        else:
            if t == "REQUEST" and r["dir"] == "out" and r.get("peer") == source:
                c["asked"] += 1
            elif t == "DATA" and r["dir"] == "in" and r.get("ann") in ("hit", "late"):
                c[r["ann"] + "s"] += 1
            elif t in ("PushMiss", "Pruned"):
                c[t.lower() + "s" if t == "PushMiss" else "prunes"] += 1
    return per


def diverge(a):
    runs = []
    for path in a.runs:
        a.rows = path
        runs.append(load(a))
    per = [per_second(rows, a.source) for rows in runs]
    keys = ("copies", "busy%", "pushes", "answers", "asked", "hits", "lates", "pushmisss", "prunes")
    print("      " + "  ".join(f"{k:>15s}" for k in keys) + "   (a / b)")
    parted = None
    for k in sorted(set(per[0]) | set(per[1])):
        x, y = per[0][k], per[1][k]
        print(f"{k:5d}s " + "  ".join(f"{x[f]:>7d}/{y[f]:<7d}" for f in keys))
        # The outcomes: on-time pushes, and the seed's answers.
        if parted is None and any(
            max(x[f], y[f]) > 100 and max(x[f], y[f]) > DIVERGE_RATIO * max(min(x[f], y[f]), 1)
            for f in ("hits", "answers")
        ):
            parted = k
    print(f"-- they part in second {parted}")
    if parted is None:
        return
    lo, hi = (parted - 2) * 1000, (parted + 1) * 1000
    kinds = [
        collections.Counter(
            (r["node"], r["type"], r.get("rule") or r.get("why") or r.get("action") or "")
            for r in rows if lo <= r["t"] < hi and r["dir"] == "tap"
            and r["type"] not in ("Upload", "RequestRound", "DataSent", "PushHit", "PushMiss")
        )
        for rows in runs
    ]
    for name, mine, other in (("a", kinds[0], kinds[1]), ("b", kinds[1], kinds[0])):
        extra = {k: v - other[k] for k, v in mine.items() if v > other[k]}
        print(f"-- decisions in {parted - 2}-{parted + 1} s that {name} has beyond the other (node, kind, rule), by count:")
        for k, v in sorted(extra.items(), key=lambda kv: -kv[1])[:20]:
            print(f"   {v:5d} {k}")


# `bytes`: each message type's class, and frame-size buckets (bytes; 1448
# is a TCP packet's payload).
BYTE_CLASSES = {
    "INTEGRITY": "integrity", "SIGNED_INTEGRITY": "signed", "HAVE": "have", "ACK": "ack",
    "REQUEST": "request", "CANCEL": "request", "PING": "ping", "PONG": "ping",
    "STREAM_REQUEST": "stream", "STREAM_CANCEL": "stream", "STREAM_OPEN": "stream",
    "STREAM_CLOSE": "stream", "ANNOUNCE": "announce", "HANDSHAKE": "session", "RESTART": "session",
    "CHOKE": "session", "UNCHOKE": "session", "HAVE_PACE": "session", "UNKNOWN_CHANNEL": "other",
    "MALFORMED": "other",
}
CLASS_ORDER = ("payload", "data_hdr", "integrity", "signed", "have", "ack", "request", "ping", "stream",
               "announce", "session", "headers", "other")
FRAME_BUCKETS = (64, 256, 1448, 4096)


class Bytes:
    """One side of some traffic: bytes by class, and the frames: their
    count, bytes and sizes, and the messages and message bytes in them (a
    frame's channel headers are its bytes less its messages')."""

    def __init__(self):
        self.cls = collections.Counter()
        self.frames = self.frame_bytes = self.msg_bytes = self.msgs = 0
        self.buckets = [0] * (len(FRAME_BUCKETS) + 1)

    def add(self, o):
        self.cls.update(o.cls)
        self.frames += o.frames
        self.frame_bytes += o.frame_bytes
        self.msg_bytes += o.msg_bytes
        self.msgs += o.msgs
        self.buckets = [x + y for x, y in zip(self.buckets, o.buckets)]

    def row(self, r):
        t = r["type"]
        if t == "FRAME":
            self.frames += 1
            self.frame_bytes += r["len"]
            self.buckets[next((i for i, b in enumerate(FRAME_BUCKETS) if r["len"] <= b), len(FRAME_BUCKETS))] += 1
            return
        if t == "DATA":
            data = r.get("data", 0)
            self.cls["payload"] += data
            self.cls["data_hdr"] += r["len"] - data
        else:
            self.cls[BYTE_CLASSES.get(t, "other")] += r["len"]
        self.msg_bytes += r["len"]
        self.msgs += t not in ("UNKNOWN_CHANNEL", "MALFORMED")

    def line(self, secs, nodes=1):
        cls = self.cls + collections.Counter(headers=self.frame_bytes - self.msg_bytes)
        total = max(self.frame_bytes, 1)
        n = max(self.frames, 1)
        shares = " ".join(f"{100 * cls[c] / total:{len(c)}.1f}" for c in CLASS_ORDER)
        return (f"{self.frame_bytes / 1e3 / secs / nodes:8.1f} {shares} | {self.frames / secs / nodes:8.1f}"
                f" {self.frame_bytes / n:6.0f} {self.msgs / n:6.1f}  "
                + " ".join(f"{100 * b / n:5.1f}" for b in self.buckets))


def bytes_header(first):
    sizes = " ".join(f"<={b}" for b in FRAME_BUCKETS) + f" >{FRAME_BUCKETS[-1]}"
    return f"{first} {'KB/s':>8s} {' '.join(CLASS_ORDER)} | frames/s mean_B msgs/f  {sizes} (%)"


def bytes_(a):
    """Where a capture's bytes go: per role and direction (or per peer of
    `--node`, or per `--step` seconds), the KB/s each node moved in ppspp
    frames, each class's share of them, and the frames: their rate, mean
    size, messages each and sizes. Streams the rows: captures are large."""
    lo = a.from_ * 1000 if a.from_ is not None else float("-inf")
    hi = a.until * 1000 if a.until is not None else float("inf")
    step = a.step * 1000 if a.step else None
    groups = collections.defaultdict(Bytes)
    nodes, pushers = set(), set()
    t0 = t1 = None
    with open(a.rows) as f:
        for line in f:
            r = json.loads(line)
            if not lo <= r["t"] <= hi:
                continue
            t0 = r["t"] if t0 is None else t0
            t1 = r["t"]
            nodes.add(r["node"])
            if r["dir"] not in ("in", "out") or "len" not in r or (a.node and r["node"] != a.node):
                continue
            if r["type"] == "DATA" and r["dir"] == "out" and r.get("ann") == "push":
                pushers.add(r["node"])
            sub = int(r["t"] // step) if step else r.get("peer") if a.node else None
            groups[(r["node"], r["dir"], sub)].row(r)
    if not any(g.frames for g in groups.values()):
        sys.exit("no FRAME rows: reduce the dumps with --all")
    secs = max((t1 - t0) / 1000, 1e-3)

    def role(n):
        # The ppspp sim's labels add the seed's site ("Seed Chicago Cable").
        seed = n.lower() == a.seed.lower() or n.lower().startswith(a.seed.lower() + " ")
        return "seed" if seed else "relay" if n in pushers else "leaf"

    folded = collections.defaultdict(Bytes)
    for (n, d, sub), g in groups.items():
        folded[(sub, d) if a.node or step else (role(n), d)].add(g)
    if step:
        print(f"-- {a.node or 'every node'}'s ppspp frames per {a.step:g} s")
        print(bytes_header(f"{'s':>7s} dir"))
        for (slot, d) in sorted(folded):
            print(f"{slot * a.step:7.1f} {d:>3s} {folded[(slot, d)].line(a.step)}")
        return
    if a.node:
        print(f"-- {a.node}'s ppspp frames per peer over {secs:.1f} s")
        print(bytes_header(f"{'peer':>10s} dir"))
        for (peer, d) in sorted(folded, key=lambda k: (str(k[0]), k[1])):
            print(f"{str(peer):>10s} {d:>3s} {folded[(peer, d)].line(secs)}")
        return
    roles = collections.Counter(role(n) for n in nodes)
    print(f"-- ppspp frames by role over {secs:.1f} s ({dict(roles)}; relays sent pushes), per node of the role")
    print(bytes_header(f"{'role':>6s} dir"))
    for key in sorted(folded, key=lambda k: (("seed", "relay", "leaf").index(k[0]), k[1])):
        print(f"{key[0]:>6s} {key[1]:>3s} {folded[key].line(secs, max(roles[key[0]], 1))}")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = ap.add_subparsers(dest="cmd", required=True)

    def command(name, f, **kw):
        p = sub.add_parser(name, **kw)
        p.add_argument("--from", dest="from_", type=float, help="seconds after the start")
        p.add_argument("--until", type=float, help="seconds after the start")
        p.set_defaults(f=f)
        return p

    p = command("reduce", reduce)
    p.add_argument("dumps", nargs="+")
    p.add_argument("--all", action="store_true", help="every row, frames and HAVEs included")
    p.add_argument("--bin", help="the ppspp-capture binary")
    for name, f in (("seed", seed), ("prunes", prunes)):
        p = command(name, f)
        p.add_argument("rows")
        p.add_argument("--node", default="seed")
        p.add_argument("--step", type=float, default=0.5, help="seconds between meter readings")
        p.add_argument("--first", type=int, default=10, help="prunes to list")
    for name, f in (("queues", queues), ("node", node)):
        p = command(name, f)
        p.add_argument("rows")
        p.add_argument("node")
        p.add_argument("--step", type=float, default=0.5, help="seconds between samples")
        p.add_argument("--first", type=int, default=6, help="admissions to list")
    p = command("links", links)
    p.add_argument("rows")
    p.add_argument("node")
    p.add_argument("peers", nargs="+")
    p.add_argument("--step", type=float, default=0.5, help="seconds between samples")
    p = command("paths", paths)
    p.add_argument("rows")
    p.add_argument("node")
    p.add_argument("peers", nargs="*")
    p.add_argument("--step", type=float, default=1.0, help="seconds between samples")
    p = command("pulls", pulls)
    p.add_argument("rows")
    p.add_argument("--source", default="seed")
    p = command("answers", answers)
    p.add_argument("rows")
    p.add_argument("--node", default="seed")
    p = command("gate", gate)
    p.add_argument("rows")
    p.add_argument("--node", default="seed")
    p = command("holders", holders)
    p.add_argument("rows")
    p.add_argument("--node", default="seed")
    p.add_argument("--holder", help="one holder only")
    p = command("bytes", bytes_)
    p.add_argument("rows")
    p.add_argument("--seed", default="seed", help="the seed's label, or its first word, in any case")
    p.add_argument("--node", help="one node's frames per peer")
    p.add_argument("--step", type=float, help="seconds per line instead (with --node, that node's)")
    p = command("diverge", diverge)
    p.add_argument("runs", nargs=2)
    p.add_argument("--source", default="seed")
    for name, f in zip(VERDICT_COMMANDS, (series, collapse, map_, pair, track, delays)):
        p = sub.add_parser(name)
        p.set_defaults(f=f)
        p.add_argument("runs", nargs="+")
        p.add_argument("--where", action="append", help="K=V: runs whose env or row has it")
    p = sub.choices["series"]
    p.add_argument("--every", type=int, default=1, help="seconds between lines")
    p = sub.choices["map"]
    p.add_argument("--group", help="the env keys of a cell (default: every key that varies)")
    p = sub.choices["pair"]
    p.add_argument("--a", required=True, help="K=V[,K=V]: the runs of arm A")
    p.add_argument("--b", required=True, action="append", help="K=V[,K=V]: the runs of an arm B")
    p.add_argument("--ignore", help="K,...: settings that don't tell cells apart")
    p.add_argument("--metric", action="append", help="F: also compare the row field F")
    p = sub.choices["delays"]
    p.add_argument("--group", help="the env keys of a cell (default: every key that varies)")
    p = sub.choices["track"]
    p.add_argument("--group", help="the env keys of a cell (default: every key that varies)")
    p.add_argument("--runs", dest="runs_", action="store_true", help="one line per run")
    a = ap.parse_args()
    try:
        a.f(a)
    except BrokenPipeError:
        pass


# The commands over runs, which read a farm matrix's tables from hack/sim-db's
# cache in the duckdb module (hack/'s uv venv); the capture tools run under
# any python3.
VERDICT_COMMANDS = ("series", "collapse", "map", "pair", "track", "delays")

if __name__ == "__main__":
    if sys.argv[1:2] and sys.argv[1] in VERDICT_COMMANDS:
        from simlab import bootstrap

        bootstrap.engine(__file__)
    main()
