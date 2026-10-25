#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Reads ppspp captures of a swarm: what its push trees decided, and why.

  hack/capture/analyze.py reduce DUMP|DIR... [--from S] [--until S] [--all]
                                             [--bin PATH] > run.jsonl
  hack/capture/analyze.py seed   run.jsonl [--node NAME] [--from S] [--until S]
  hack/capture/analyze.py queues run.jsonl NODE [--from S] [--until S] [--step S]
  hack/capture/analyze.py links  run.jsonl NODE PEER... [--from S] [--until S]
                                 [--step S]
  hack/capture/analyze.py prunes run.jsonl [--node NAME] [--from S] [--until S]
  hack/capture/analyze.py pulls  run.jsonl [--source NAME] [--from S] [--until S]
  hack/capture/analyze.py node   run.jsonl NODE [--from S] [--until S]

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
  prunes  stale pushes and push waits at a node, the prunes by rule with
          their numbers, and the misses and pushes behind the first prune
  pulls   where the viewers' requests went (the source or other viewers),
          the request windows behind them, and request timeouts
  node    one node's decisions and traffic per second, its first admissions
          with their numbers, and its upload meter
"""

import argparse
import collections
import json
import os
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
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
    sys.exit(subprocess.call(cmd + a.dumps))


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


def med(v):
    v = sorted(v)
    return v[len(v) // 2] if v else None


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
    p = command("pulls", pulls)
    p.add_argument("rows")
    p.add_argument("--source", default="seed")
    a = ap.parse_args()
    try:
        a.f(a)
    except BrokenPipeError:
        pass


if __name__ == "__main__":
    main()
