#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Compares a TCP bench run on the testbed with the same bench in the sim.

    hack/testbed/bench.py <run dir> [<sim log dir> ...] [--from <s>]
                          [--only <arm>,...]

<run dir> is bench.sh's output: the testbed's node-<i>.jsonl and tc-0.log,
and the sim's logs in sim/ and sim-tcp/ (more sim log dirs, as
testbed_bench --sim writes them, may follow; --only keeps the testbed and
the arms named). Each arm gets a column; each
row is a quantile (p10/p50/p90/p99/max, in ms unless named) over the data
records and samples from --from seconds on (default 2), pooled over the
links that carry the same schedule:

  owd     a record's write start to its last byte read
  lag     its due time to its last byte read
  block   its write start to its last byte in the socket
  rtt     a probe's write start to its echo read
  rtt@ph  the same by the probe's phase in its burst period (ms)
  unsent  the socket's unsent bytes (KiB)
  cwnd    its congestion window (KiB)
  flight  its bytes sent and unacknowledged (KiB)
  upq     the seed's uplink queue: tc's tbf backlog over its rate at 10 Hz,
          or the model's every 10 ms
  rate    bytes read per second per link (KB/s), mean
  drops   the seed's packets its uplink queue dropped over the whole run
          (testbed: tc's tbf; sim: the window model's segments)
"""

import json
import os
import re
import sys
from collections import defaultdict


def quantiles(xs, qs=(0.1, 0.5, 0.9, 0.99)):
    if not xs:
        return [None] * (len(qs) + 1)
    xs = sorted(xs)
    out = [xs[min(len(xs) - 1, int(q * len(xs)))] for q in qs]
    return out + [xs[-1]]


def load(d):
    """{node: [events]} from a dir of node-<i>.jsonl."""
    nodes = {}
    for f in sorted(os.listdir(d)):
        m = re.fullmatch(r"node-(\d+)\.jsonl", f)
        if not m:
            continue
        evs = []
        with open(os.path.join(d, f)) as fh:
            for line in fh:
                line = line.strip()
                if line.startswith("{"):
                    try:
                        evs.append(json.loads(line))
                    except json.JSONDecodeError:
                        pass
        nodes[int(m.group(1))] = evs
    return nodes


def settings(run):
    s = {}
    p = os.path.join(run, "settings")
    if os.path.exists(p):
        for line in open(p):
            k, _, v = line.strip().partition("=")
            s[k] = v
    return s


def schedule(run):
    """[(kind, from, to, start, period, bytes, write)] from the schedule."""
    out = []
    for line in open(os.path.join(run, "schedule")):
        w = line.split("#")[0].split()
        if not w:
            continue
        if w[0] == "burst":
            out.append(("burst", int(w[1]), int(w[2]), int(w[4]), int(w[5]), int(w[6])))
        elif w[0] == "probe":
            out.append(("probe", int(w[1]), int(w[2]), int(w[3]), int(w[4]), 0))
    return out


def classes(sched):
    """{(from, to): class name}: links with the same bursts share a class."""
    sig = defaultdict(list)
    for e in sched:
        if e[0] == "burst":
            sig[(e[1], e[2])].append(f"{e[4]}B/{e[3]}ms")
    names = {}
    for link, s in sig.items():
        names[link] = "+".join(sorted(s))
    return names


def tc_queue(run, start_ms, s):
    """The seed's tbf queue in ms, [(t_us, ms)], from tc-0.log."""
    p = os.path.join(run, "tc-0.log")
    if not os.path.exists(p) or start_ms is None:
        return []
    up = float(s.get("SEED_UP_MBIT", "10"))
    steps = []
    for st in s.get("SEED_UP_STEPS", "").split():
        a, _, m = st.partition(":")
        steps.append((float(a), float(m)))
    out = []
    for line in open(p):
        w = line.split()
        if not w:
            continue
        t = (int(w[0]) - start_ms) * 1000
        backs = re.findall(r"backlog (\d+)b", line)
        if len(backs) < 2:
            continue
        rate = up
        for a, m in steps:
            if t >= a * 1e6:
                rate = m
        out.append((t, int(backs[1]) * 8 / (rate * 1e6) * 1000))
    return sorted(out)


def tc_drops(run):
    """The seed's tbf drops over the run, from tc-0.log's last sample."""
    p = os.path.join(run, "tc-0.log")
    if not os.path.exists(p):
        return None
    last = max(open(p), key=lambda l: int(l.split()[0]) if l.split() else 0)
    d = re.findall(r"dropped (\d+)", last)
    return int(d[1]) if len(d) > 1 else None


def metrics(nodes, sched, cls, t0_us, queue=None):
    m = defaultdict(list)
    period = {}
    for e in sched:
        if e[0] == "burst":
            period[(e[1], e[2])] = e[3]
    rbytes = defaultdict(int)
    tmax = 0
    # The sim's uplink queue, from one link's samples.
    first = {i: min((e["to"] for e in evs if e.get("ev") == "s"), default=None) for i, evs in nodes.items()}
    for i, evs in nodes.items():
        for e in evs:
            ev = e.get("ev")
            if ev == "r" and e["k"] == 0:
                link = (e["from"], i)
                c = cls.get(link, "?")
                tmax = max(tmax, e["got"])
                if e["at"] < t0_us:
                    continue
                m[(c, "owd")].append((e["got"] - e["at"]) / 1000)
                m[(c, "lag")].append((e["got"] - e["due"]) / 1000)
                rbytes[(c, link)] += e["len"]
            elif ev == "r" and e["k"] == 2:
                link = (i, e["from"])
                c = cls.get(link, "idle")
                if e["at"] < t0_us:
                    continue
                rtt = (e["got"] - e["at"]) / 1000
                m[(c, "rtt")].append(rtt)
                p = period.get(link)
                if p:
                    ph = (e["at"] / 1000) % p
                    b = int(ph // (p / 5)) * (p // 5)
                    m[(c, f"rtt@{b:>3}")].append(rtt)
            elif ev == "w" and e["k"] == 0:
                c = cls.get((i, e["to"]), "?")
                if e["at"] < t0_us:
                    continue
                m[(c, "block")].append((e["done"] - e["at"]) / 1000)
            elif ev == "s":
                c = cls.get((i, e["to"]), "idle")
                if e["t"] < t0_us:
                    continue
                m[(c, "unsent")].append(e["unsent"] / 1024)
                m[(c, "cwnd")].append(e["cwnd"] / 1024)
                m[(c, "flight")].append(e.get("unacked", e.get("in_flight", 0)) / 1024)
                if "upq" in e and i == 0 and e["to"] == first.get(i):
                    m[("node 0", "upq")].append(e["upq"] / 1000)
    for t, ms in queue or []:
        if t >= t0_us:
            m[("node 0", "upq")].append(ms)
    for e in nodes.get(0, []):
        if e.get("ev") == "drops":
            m[("node 0", "drops")] = [e["queue_drops"]]
    span = max(1.0, (tmax - t0_us) / 1e6)
    per = defaultdict(list)
    for (c, link), b in rbytes.items():
        per[c].append(b / span / 1000)
    for c, v in per.items():
        m[(c, "rate")] = v
    return m


def fmt(v):
    if v is None:
        return "-"
    if abs(v) >= 100:
        return f"{v:.0f}"
    return f"{v:.1f}"


def main():
    args = sys.argv[1:]
    t0 = 2.0
    if "--from" in args:
        i = args.index("--from")
        t0 = float(args[i + 1])
        del args[i : i + 2]
    only = None
    if "--only" in args:
        i = args.index("--only")
        only = args[i + 1].split(",")
        del args[i : i + 2]
    run = args[0]
    extra = args[1:]
    s = settings(run)
    sched = schedule(run)
    cls = classes(sched)
    arms = [("testbed", run)]
    for d in dict.fromkeys(["sim", "sim-tcp"] + extra + (only or [])):
        p = d if os.path.isdir(d) and d in extra else os.path.join(run, d)
        if os.path.isdir(p):
            arms.append((os.path.basename(p.rstrip("/")), p))
    if only:
        arms = [a for a in arms if a[0] == "testbed" or a[0] in only]
    ms = []
    for name, d in arms:
        nodes = load(d)
        start = None
        if name == "testbed":
            for e in nodes.get(0, []):
                if e.get("ev") == "start":
                    start = e["ms"]
        q = tc_queue(run, start, s) if name == "testbed" else None
        m = metrics(nodes, sched, cls, int(t0 * 1e6), q)
        if name == "testbed":
            m[("node 0", "drops")] = [tc_drops(run)]
        ms.append(m)
    keys = sorted(set().union(*[m.keys() for m in ms]))
    print(f"{run}: {' '.join(f'{k}={v}' for k, v in s.items() if k != 'NODE_ARGS')}")
    print(f"from {t0} s; columns: " + "  ".join(n for n, _ in arms))
    for c in sorted({k[0] for k in keys}):
        print(f"\n{c}")
        for k in [k for k in keys if k[0] == c]:
            name = k[1]
            row = f"  {name:<8}"
            for m in ms:
                xs = m.get(k, [])
                if name == "drops":
                    row += f" | {xs[0] if xs else '-':>6}"
                elif name == "rate":
                    row += f" | {fmt(sum(xs) / len(xs)) if xs else '-':>6}"
                else:
                    qs = quantiles(xs)
                    row += " | " + " ".join(f"{fmt(q):>5}" for q in qs)
            print(row)


if __name__ == "__main__":
    main()
