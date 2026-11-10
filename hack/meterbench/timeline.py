#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""One meter bench run over time.

    hack/meterbench/timeline.py <run dir> [--every S] [--meters a,b]

Each row is a span of --every seconds (default 5): the true uplink (kB/s),
the copies, the copy rate (kB/s), what the node wrote (kB/s), each meter's
mean reading (kB/s), the uplink queue's p50/p90 (ms), and the pushed units'
lag p50/p99 at the children whose downlinks take a copy (ms), and at the
narrow ones apart.
"""

import json
import os
import sys
from collections import defaultdict


def q(xs, p):
    if not xs:
        return None
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]


def main():
    args = sys.argv[1:]
    every = 5.0
    meters = None
    if "--every" in args:
        i = args.index("--every")
        every = float(args[i + 1])
        del args[i : i + 2]
    if "--meters" in args:
        i = args.index("--meters")
        meters = args[i + 1].split(",")
        del args[i : i + 2]
    run = args[0]
    cond = json.load(open(os.path.join(run, "condition.json")))
    narrow = {i + 1 for i, nb in enumerate(cond["neighbours"]) if nb["down"] is not None}
    print(
        f"up {cond['up'] / 125000:.1f} Mbit/s, stream {cond['kbit']} kbit/s, {len(cond['neighbours'])} links, "
        f"queue {cond['queue_ms']:.0f} ms, class {cond['class']}, narrow {sorted(narrow)}"
    )
    span = lambda t: int(t / every)
    caps = defaultdict(lambda: defaultdict(list))
    ks, rates, wrote, queue = defaultdict(list), defaultdict(list), defaultdict(int), defaultdict(list)
    for line in open(os.path.join(run, "node-0.jsonl")):
        if not line.startswith("{"):
            continue
        m = json.loads(line)
        t = m.get("t", 0) / 1e6
        if m["ev"] == "m" and m["cap"] is not None:
            caps[m["meter"]][span(t)].append(m["cap"])
        elif m["ev"] == "k":
            ks[span(t)].append(m["copies"])
            rates[span(t)].append(m["rate"])
        elif m["ev"] == "b":
            wrote[span(t - 0.5)] += sum(m[k] for k in ("push", "answer", "control", "probe", "pad"))
        elif m["ev"] == "q":
            queue[span(t)].append(m["wait"] / 1e3)
    lags, slow = defaultdict(list), defaultdict(list)
    for name in os.listdir(run):
        if not name.startswith("node-") or name == "node-0.jsonl":
            continue
        i = int(name[5:-6])
        for line in open(os.path.join(run, name)):
            if line.startswith('{"ev":"u"'):
                u = json.loads(line)
                (slow if i in narrow else lags)[span(u["at"] / 1e6)].append((u["last"] - u["at"]) / 1e3)
    names = meters or sorted(caps)
    head = f"{'t':>5}{'up':>7}{'k':>5}{'rate':>7}{'wrote':>7}" + "".join(f"{n[:9]:>10}" for n in names)
    print(head + f"{'q50':>6}{'q90':>6}{'lag50':>7}{'lag99':>7}{'slow99':>8}")
    for s in range(int(cond["secs"] / every)):
        t = s * every
        up = cond["up"]
        for at, r in cond["up_steps"]:
            if at <= t:
                up = r
        k = q(ks[s], 0.5)
        rate = q(rates[s], 0.5)
        row = f"{t:5.0f}{up / 1e3:7.0f}{k if k is not None else '-':>5}{(rate or 0) / 1e3:7.0f}{wrote[s] / every / 1e3:7.0f}"
        for n in names:
            v = caps[n][s]
            row += f"{sum(v) / len(v) / 1e3:10.0f}" if v else f"{'-':>10}"
        f = lambda x: f"{x:6.0f}" if x is not None else f"{'-':>6}"
        row += f(q(queue[s], 0.5)) + f(q(queue[s], 0.9)) + " " + f(q(lags[s], 0.5)) + " " + f(q(lags[s], 0.99))
        row += "  " + f(q(slow[s], 0.99))
        print(row)


if __name__ == "__main__":
    main()
