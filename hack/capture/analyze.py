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

Runs, second by second (no capture needed):

  hack/capture/analyze.py series   RUNS... [--where K=V...] [--every S]
  hack/capture/analyze.py collapse RUNS... [--where K=V...]
  hack/capture/analyze.py map      RUNS... [--where K=V...] [--group K,...]

RUNS are farm matrix dirs (`target/sim-farm/<matrix>`: its rows.jsonl, and
a testbed matrix's run dirs), rows.jsonl files, stdout with `ROW {json}`
lines, or testbed run dirs (`hack/testbed/run.sh`'s out dir). A sim row
carries `series` with `STRIMS_SIM_SERIES=1` (strims-sim, where
`calibration_star` always has it; `Report::series` documents the fields)
or `SIM_SERIES=1` (the ppspp sim's `sim_job`); a testbed run's comes from
its nodes' lines. `--where` keeps the runs whose env (or row field) has
the value; keys match with or without the STRIMS_SIM_CAL_, STRIMS_SIM_ or
SIM_ prefix.

  series    per second: viewers present, on-time pushes against the chunks
            the viewers needed, the median viewer's delivery, the swarm's
            lag, prunes, parents, and the seed's meter, uplink, busy share,
            copies and share of its chunks sent as pushes
  collapse  one line per run: its verdicts, onsets, course and feasibility
  map       per cell (the runs that share every setting but the seed): the
            share of runs whose trees failed, delivery collapsed and latency
            collapsed, the share absorbing, the median onsets, and whether
            the cell is feasible

The verdicts, judged over the stream's seconds (not the run's tail):

- A viewer's lag in a second is the p99 of the frames it read then; in a
  second it reads none, its lag from the last second grows by a second (it
  falls behind live). The swarm's lag is the LAG_QUANTILE of its viewers'.
- Trees fail when, for TREES_SECS or more in a row, under TREES_SHARE of
  the chunks the viewers needed arrive as pushes before their deadline
  (hits), averaged over TREES_SMOOTH seconds. Counting against the chunks
  needed, not the pushes that came, makes a swarm with no parents left a
  failure rather than undefined.
- Delivery collapses when the median viewer's frames over its last
  DELIVERY_SECS come to under DELIVERY_SHARE of the stream.
- Latency collapses when the swarm's lag stays over LAG_HIGH_MS for
  LAG_HIGH_SECS, or for LAG_HIGH_SECS in a row its least-squares slope over
  the last LAG_SLOPE_SECS passes LAG_SLOPE ms a second with the lag over
  LAG_SLOPE_FLOOR_MS (a spike that turns back sooner is no rise).
- A verdict's onset is the first second of its run of bad seconds, or for
  delivery the first second whose window falls short.
- A collapsed run recovers once all three are healthy for RECOVER_SECS. It
  is absorbing when it hasn't recovered by the stream's end, or within
  ABSORB_SECS of the trigger going away (the seed's uplink stepping back
  up).
- A cell is feasible when the seed's lowest uplink carries one copy of the
  stream with OVERHEAD and the seed's spare plus the viewers' uplinks
  carry every viewer's copy: a collapse no tree could avoid isn't the
  protocol's. `map` marks cells feasible only at full uplink as "step".
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


# The verdicts' thresholds (see the module docs).
TREES_SHARE = 0.5
TREES_SMOOTH = 3
TREES_SECS = 10
DELIVERY_SHARE = 0.95
DELIVERY_SECS = 10
LAG_QUANTILE = 0.9
LAG_HIGH_MS = 3000
LAG_HIGH_SECS = 5
LAG_SLOPE = 50
LAG_SLOPE_SECS = 20
LAG_SLOPE_FLOOR_MS = 1000
RECOVER_SECS = 20
ABSORB_SECS = 60
# The wire's bytes beyond the stream's, as a share of them, by chunk size:
# a lone viewer of a 100 Mbit seed in calibration_star (64 KiB munros at
# 1 KiB, 32 KiB at 4 KiB), its data, HAVEs, requests and integrity, with
# TCP's headers.
OVERHEAD = {1024: 0.34, 4096: 0.085}
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
    """A testbed run dir's series, in `Report::series`'s fields."""
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


def runs(paths):
    """(label, env, row, series) per run in `paths` (see the module docs)."""
    for p in map(Path, paths):
        if p.is_dir() and (p / "node-0.jsonl").exists():
            yield p.name, {}, {}, testbed_series(p, {})
            continue
        rows = p / "rows.jsonl" if p.is_dir() else p
        with open(rows) as f:
            for line in f:
                line = line.strip()
                if line.startswith("ROW "):
                    line = line[4:]
                if not line.startswith("{"):
                    continue
                r = json.loads(line)
                if "row" in r:
                    env = {**r.get("env", {}), "arm": r.get("arm", "")}
                    if r.get("kind") == "testbed":
                        env["repeat"] = r.get("repeat")
                        yield r["id"], env, r["row"], testbed_series(rows.parent / r["run_dir"], r.get("env", {}))
                    elif "series" in r["row"]:
                        yield r["id"], env, r["row"], r["row"]["series"]
                elif "series" in r:
                    yield f"{r.get('test', r.get('scenario'))} seed {r.get('seed')}", {"seed": r.get("seed")}, r, r["series"]


def matches(env, row, where):
    for w in where or ():
        k, _, v = w.partition("=")
        have = {short(x): str(y) for x, y in {**row, **env}.items()}
        if have.get(short(k)) != v:
            return False
    return True


def quantile(v, q):
    v = sorted(v)
    return v[round((len(v) - 1) * q)] if v else None


def lags(s):
    """Each viewer's lag per second, falling a second behind in a second it
    reads nothing (None before its first frame)."""
    out = []
    for lag, frames in zip(s["lag"], s["frames"]):
        cur, v = None, []
        for l, f in zip(lag, frames):
            if f and l >= 0:
                cur = l
            elif cur is not None:
                cur += 1000
            v.append(cur)
        out.append(v)
    return out


def measures(s):
    """The per-second measures the verdicts read, over the stream's seconds."""
    n = min(s["secs"], len(s["present"]))
    per_s = s["rate"] / s["chunk"]
    share = [s["hits"][k] / (s["present"][k] * per_s) if s["present"][k] else None for k in range(n)]
    smooth = []
    for k in range(n):
        w = [x for x in share[max(0, k - TREES_SMOOTH // 2): k + TREES_SMOOTH // 2 + 1] if x is not None]
        smooth.append(sum(w) / len(w) if w else None)
    firsts = [next((i for i, f in enumerate(frames) if f), None) for frames in s["frames"]]
    deliv = []
    for k in range(n):
        d = [
            sum(frames[k - DELIVERY_SECS + 1: k + 1]) * s["frame"] / DELIVERY_SECS / s["rate"]
            for frames, first in zip(s["frames"], firsts)
            if first is not None and k >= first + DELIVERY_SECS - 1
        ]
        deliv.append(med(d))
    vl = lags(s)
    lag = [quantile([v[k] for v in vl if v[k] is not None], LAG_QUANTILE) for k in range(n)]
    return n, smooth, deliv, lag


def slope(ys):
    xs = range(len(ys))
    mx, my = sum(xs) / len(ys), sum(ys) / len(ys)
    den = sum((x - mx) ** 2 for x in xs)
    return sum((x - mx) * (y - my) for x, y in zip(xs, ys)) / den if den else 0


def runs_of(bad, need):
    """The first second of the first run of `need` bad seconds in a row."""
    k0 = None
    for k, b in enumerate(bad):
        if b:
            k0 = k if k0 is None else k0
            if k - k0 + 1 >= need:
                return k0
        else:
            k0 = None
    return None


def verdict(s):
    n, smooth, deliv, lag = measures(s)
    trees_bad = [x is not None and x < TREES_SHARE for x in smooth]
    deliv_bad = [x is not None and x < DELIVERY_SHARE for x in deliv]
    high = [x is not None and x > LAG_HIGH_MS for x in lag]
    rise = []
    for k in range(n):
        w = [x for x in lag[max(0, k - LAG_SLOPE_SECS + 1): k + 1] if x is not None]
        rise.append(len(w) >= LAG_SLOPE_SECS and lag[k] > LAG_SLOPE_FLOOR_MS and slope(w) > LAG_SLOPE)
    trees = runs_of(trees_bad, TREES_SECS)
    delivery = next((k for k, b in enumerate(deliv_bad) if b), None)
    high_at = runs_of(high, LAG_HIGH_SECS)
    rise_at = runs_of(rise, LAG_HIGH_SECS)
    latency = min((x for x in (high_at, rise_at) if x is not None), default=None)
    onsets = [x for x in (trees, delivery, latency) if x is not None]
    onset = min(onsets) if onsets else None
    healthy = [not (trees_bad[k] or deliv_bad[k] or high[k] or rise[k]) for k in range(n)]
    recovered = None
    if onset is not None:
        k0 = runs_of([h and k > onset for k, h in enumerate(healthy)], RECOVER_SECS)
        recovered = k0
    # The trigger goes away when the seed's uplink last steps back up.
    ups = [t for (t, u), (_, before) in zip(s["seed_up"][1:], s["seed_up"]) if u > before]
    absorbing = False
    if onset is not None:
        if recovered is None:
            absorbing = True
        elif ups and onset <= ups[-1] and recovered > ups[-1] + ABSORB_SECS:
            absorbing = True
    return {
        "trees": trees, "delivery": delivery, "latency": latency, "recovered": recovered,
        "absorbing": absorbing, "feasible": feasible(s), "lag_p50": med([x for x in lag if x is not None]),
        "lag_max": max((x for x in lag if x is not None), default=None),
    }


def feasible(s):
    """True, False, or "step": feasible at the seed's full uplink only."""
    copy = s["rate"] * (1 + OVERHEAD.get(s["chunk"], OVERHEAD[1024]))
    viewers = len(s["lag"])
    ok = lambda up: up >= copy and up + s["viewer_up"] >= viewers * copy
    lo, hi = min(u for _, u in s["seed_up"]), s["seed_up"][0][1]
    return True if ok(lo) else ("step" if ok(hi) else False)


def series(a):
    for label, env, row, s in runs(a.runs):
        if not matches(env, row, a.where):
            continue
        n, smooth, deliv, lag = measures(s)
        vl = lags(s)
        print(f"== {label} {' '.join(f'{short(k)}={v}' for k, v in sorted(env.items()) if v not in (None, ''))}")
        print("    s viewers ontime deliv   lag_ms pruned parents  seed: cap_kB avail_kB busy copies sent pushed%")
        for k in range(0, len(s["present"]), a.every):
            f = lambda x, fmt: format(x, fmt) if x is not None else "-"
            sent = s["sent"][k]
            print(
                f"{k:5d} {s['present'][k]:7d} {f(smooth[k] if k < n else None, '6.2f')} {f(deliv[k] if k < n else None, '5.2f')}"
                f" {f(quantile([v[k] for v in vl if v[k] is not None], LAG_QUANTILE), '8d')} {s['pruned'][k]:6d} {s['parents'][k]:7d}"
                f"  {s['capacity'][k] // 1000 if s['capacity'][k] >= 0 else '-':>12} {s['avail'][k] // 1000 if s['avail'][k] >= 0 else '-':>8}"
                f" {s['busy'][k] / 10 if s['busy'][k] >= 0 else '-':>5} {s['copies'][k]:6d} {sent:4d} {100 * s['pushed'][k] // sent if sent else '-':>7}"
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
    print(" ".join(short(k) for k in group) + " | n feasible | trees delivery latency absorbing | onsets: trees delivery latency | lag_p50")
    for cell in sorted(cells, key=lambda c: [num(x) for x in c]):
        vs = cells[cell]
        fe = "/".join(sorted({str(v["feasible"]) for v in vs}))
        print(
            f"{' '.join(cell)} | {len(vs)} {fe} | {pct(vs, lambda v: v['trees'] is not None)} {pct(vs, lambda v: v['delivery'] is not None)}"
            f" {pct(vs, lambda v: v['latency'] is not None)} {pct(vs, lambda v: v['absorbing'])} |"
            f" {onset(vs, 'trees')} {onset(vs, 'delivery')} {onset(vs, 'latency')} | {med([v['lag_p50'] for v in vs if v['lag_p50'] is not None])}"
        )


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
    for name, f in (("series", series), ("collapse", collapse), ("map", map_)):
        p = sub.add_parser(name)
        p.set_defaults(f=f)
        p.add_argument("runs", nargs="+")
        p.add_argument("--where", action="append", help="K=V: runs whose env or row has it")
    p = sub.choices["series"]
    p.add_argument("--every", type=int, default=1, help="seconds between lines")
    p = sub.choices["map"]
    p.add_argument("--group", help="the env keys of a cell (default: every key that varies)")
    a = ap.parse_args()
    try:
        a.f(a)
    except BrokenPipeError:
        pass


if __name__ == "__main__":
    main()
