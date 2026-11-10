#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Scores the upload meter bench's runs (strims-sim's bench::meter, run by
the meter_bench example) against the truth.

    hack/meterbench/score.py <root> [--drivers a,b] [--meters a,b]
                             [--cells c,...] [--by class|cell] [--warm S]

<root> holds meter_bench sim's runs, <cell>/s<seed>/<driver>/. Two tables:

tracking, per meter (every meter a run read, driving or alongside) and
class, over runs and their seeds:
  e10/e50/e90  log2(reading / true uplink), quantiles over the readings
               from --warm seconds (default 10) on, outside the 3 s after
               a step of the uplink (pooled per run, then the median over
               runs)
  in25         the share of readings within 25 % of the uplink
  low          the share under 0.6 of it (a false drop where the uplink
               held: competition, latency, routes, loss, jitter)
  high         the share over 1.5 of it
  drop         seconds from a fall of the uplink to a reading under 1.2
               of the new rate (median over falls; '>' censored at the
               next step), and rise: to over 0.8 of a risen one
  over         the share of readings that say one more copy fits (what
               the node wrote in the last second plus a copy under the
               reading) where it doesn't (over 0.95 of the uplink)
  under        the share that say it doesn't where it plainly does
               (under 0.6 of the uplink)

closed loop, per driving meter and class:
  lag50/lag99  pushed units' lag at their children (signing to the last
               byte read), ms, pooled per run, median over runs
  late         the share of pushed units later than 1 s
  ans99        answers' lag p99, ms
  copies       mean copies, and the oracle's (the same cell and seed
               driven by `oracle`, when it ran)
  probe        probe and padding bytes as a share of all the node wrote
"""

import json
import math
import os
import sys
from collections import defaultdict


def flag(args, name, default=None):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i : i + 2]
        return v
    return default


def q(xs, p):
    if not xs:
        return None
    xs = sorted(xs)
    return xs[min(len(xs) - 1, int(p * len(xs)))]


def med(xs):
    xs = [x for x in xs if x is not None]
    if not xs:
        return None
    return q(xs, 0.5)


def up_at(cond, t):
    r = cond["up"]
    for at, rate in cond["up_steps"]:
        if at <= t:
            r = rate
    return r


def steps(cond):
    """(time, old, new) for each step of the uplink."""
    out = []
    r = cond["up"]
    for at, rate in cond["up_steps"]:
        out.append((at, r, rate))
        r = rate
    return out


def load(run):
    with open(os.path.join(run, "condition.json")) as f:
        cond = json.load(f)
    readings = defaultdict(list)
    checks, bytes_, queue = [], [], []
    with open(os.path.join(run, "node-0.jsonl")) as f:
        for line in f:
            if not line.startswith('{"ev":"'):
                continue
            ev = line[7:9]
            if ev == 'm"':
                m = json.loads(line)
                readings[m["meter"]].append((m["t"] / 1e6, m["cap"]))
            elif ev == 'k"':
                checks.append(json.loads(line))
            elif ev == 'b"':
                bytes_.append(json.loads(line))
            elif ev == 'q"':
                m = json.loads(line)
                queue.append((m["t"] / 1e6, m["wait"] / 1e3))
    units, answers, narrow_units = [], [], []
    narrow = {i + 1 for i, nb in enumerate(cond["neighbours"]) if nb["down"] is not None}
    for name in os.listdir(run):
        if not name.startswith("node-") or name == "node-0.jsonl":
            continue
        pool = narrow_units if int(name[5:-6]) in narrow else units
        with open(os.path.join(run, name)) as f:
            for line in f:
                if line.startswith('{"ev":"u"'):
                    u = json.loads(line)
                    pool.append(((u["at"]) / 1e6, (u["last"] - u["at"]) / 1e3))
                elif line.startswith('{"ev":"a"'):
                    a = json.loads(line)
                    answers.append((a["at"] / 1e6, (a["got"] - a["at"]) / 1e3))
    return cond, readings, checks, bytes_, queue, units, answers, narrow_units


def sensitivity(a, b):
    """Median |log2| of two runs' readings at the same ticks."""
    bt = {t: c for t, c in b}
    xs = [abs(math.log2(max(c, 1.0) / max(bt[t], 1.0))) for t, c in a if c is not None and bt.get(t)]
    return q(xs, 0.5)


def written_by_second(bytes_):
    """{second: bytes the node wrote that second}."""
    return {round(b["t"] / 1e6): sum(b[k] for k in ("push", "answer", "control", "probe", "pad")) for b in bytes_}


def track(cond, rs, checks, wrote, warm):
    """Tracking scores of one meter's readings in one run."""
    st = steps(cond)
    rate_at, room_at = {}, {}
    for k in checks:
        rate_at[round(k["t"] / 1e6)] = k["rate"]
        # A decision only while a neighbour could take one more copy.
        room_at[round(k["t"] / 1e6)] = k["copies"] < k.get("present", 1 << 30)
    e, in25, low, high, over, under, n = [], 0, 0, 0, 0, 0, 0
    for t, cap in rs:
        if t < warm or cap is None:
            continue
        if any(0 <= t - at < 3 for at, _, _ in st):
            continue
        c = up_at(cond, t)
        n += 1
        x = math.log2(max(cap, 1.0) / c)
        e.append(x)
        in25 += abs(cap / c - 1) <= 0.25
        low += cap < 0.6 * c
        high += cap > 1.5 * c
        s = math.floor(t)
        w = wrote.get(s, wrote.get(s - 1))
        r = rate_at.get(s) or rate_at.get(s - 1)
        room = room_at.get(s, room_at.get(s - 1, True))
        if w is not None and r and room:
            fits = w + r <= cap
            if fits and w + r > 0.95 * c:
                over += 1
            if not fits and w + r < 0.6 * c:
                under += 1
    out = {
        "e": e,
        "in25": in25 / n if n else None,
        "low": low / n if n else None,
        "high": high / n if n else None,
        "over": over / n if n else None,
        "under": under / n if n else None,
        "drop": [],
        "rise": [],
    }
    ends = [at for at, _, _ in st] + [cond["secs"]]
    for i, (at, old, new) in enumerate(st):
        nxt = ends[i + 1]
        hit = None
        for t, cap in rs:
            if t < at or t >= nxt or cap is None:
                continue
            if (new < old and cap <= 1.2 * new) or (new > old and cap >= 0.8 * new):
                hit = t - at
                break
        key = "drop" if new < old else "rise"
        out[key].append(hit if hit is not None else -(nxt - at))
    return out


def fmt(x, w=6, p=2):
    if x is None:
        return " " * (w - 1) + "-"
    return f"{x:{w}.{p}f}"


def fmt_t(xs):
    """Median of times to detect, censored ones negative."""
    if not xs:
        return "     -"
    m = q(xs, 0.5)
    if m < 0:
        return f"  >{-m:3.0f}"
    return f"{m:6.1f}"


def main():
    args = sys.argv[1:]
    drivers = flag(args, "--drivers")
    meters = flag(args, "--meters")
    cells = flag(args, "--cells")
    by = flag(args, "--by", "class")
    warm = float(flag(args, "--warm", "10"))
    runs_of = flag(args, "--runs")
    if not args:
        print(__doc__)
        sys.exit(2)
    root = args[0]
    drivers = drivers.split(",") if drivers else None
    meters = meters.split(",") if meters else None
    cells = cells.split(",") if cells else None
    tracking = defaultdict(lambda: defaultdict(list))
    loop = defaultdict(lambda: defaultdict(list))
    copies_by = {}
    pairs = {}
    for cell in sorted(os.listdir(root)):
        if cells and cell not in cells and cell.split("-")[0] not in cells:
            continue
        cdir = os.path.join(root, cell)
        if not os.path.isdir(cdir):
            continue
        for seed in sorted(os.listdir(cdir)):
            for driver in sorted(os.listdir(os.path.join(cdir, seed))):
                if drivers and driver not in drivers:
                    continue
                run = os.path.join(cdir, seed, driver)
                if not os.path.exists(os.path.join(run, "node-0.jsonl")):
                    continue
                cond, readings, checks, bytes_, queue, units, answers, narrow_units = load(run)
                group = cond["class"] if by == "class" else cell
                if cond["class"] in ("byzantine", "byzclean"):
                    pair_key = (cell.split("-", 1)[1], seed, driver)
                    pairs[(cond["class"],) + pair_key] = readings
                wrote = written_by_second(bytes_)
                for m, rs in readings.items():
                    if meters and m not in meters:
                        continue
                    s = track(cond, rs, checks, wrote, warm)
                    if runs_of == m:
                        print(
                            f"{cell}/{seed}/{driver}: e10 {fmt(q(s['e'], 0.1))} e50 {fmt(q(s['e'], 0.5))}"
                            f" e90 {fmt(q(s['e'], 0.9))} low {fmt(s['low'])} high {fmt(s['high'])}"
                            f" up {cond['up'] / 125000:.1f} kbit {cond['kbit']} n {len(cond['neighbours'])}"
                            f" queue {cond['queue_ms']:.0f}"
                        )
                    d = tracking[(m, group)]
                    d["e10"].append(q(s["e"], 0.1))
                    d["e50"].append(q(s["e"], 0.5))
                    d["e90"].append(q(s["e"], 0.9))
                    for k in ("in25", "low", "high", "over", "under"):
                        d[k].append(s[k])
                    d["drop"].extend(s["drop"])
                    d["rise"].extend(s["rise"])
                lags = [l for at, l in units if at >= warm]
                ans = [l for at, l in answers if at >= warm]
                ks = [k["copies"] for k in checks if k["t"] / 1e6 >= warm]
                total = sum(sum(b[k] for k in ("push", "answer", "control", "probe", "pad")) for b in bytes_)
                probe = sum(b["probe"] + b["pad"] for b in bytes_)
                d = loop[(driver, group)]
                d["lag50"].append(q(lags, 0.5))
                d["lag99"].append(q(lags, 0.99))
                d["slow99"].append(q([l for at, l in narrow_units if at >= warm], 0.99))
                d["late"].append(sum(l > 1000 for l in lags) / len(lags) if lags else None)
                d["ans99"].append(q(ans, 0.99))
                mean_k = sum(ks) / len(ks) if ks else None
                d["copies"].append(mean_k)
                d["probe"].append(probe / total if total else None)
                d["runs"].append((cell, seed))
                copies_by[(cell, seed, driver)] = mean_k
    print("tracking (log2 reading/uplink; shares of readings; seconds)")
    print(f"{'meter':<16}{'class':<11}{'n':>4}{'e10':>7}{'e50':>7}{'e90':>7}{'in25':>7}{'low':>7}{'high':>7}{'drop':>7}{'rise':>7}{'over':>7}{'under':>7}")
    for (m, g), d in sorted(tracking.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        print(
            f"{m:<16}{g:<11}{len(d['e50']):>4}{fmt(med(d['e10']), 7)}{fmt(med(d['e50']), 7)}{fmt(med(d['e90']), 7)}"
            f"{fmt(med(d['in25']), 7)}{fmt(med(d['low']), 7)}{fmt(med(d['high']), 7)} {fmt_t(d['drop'])} {fmt_t(d['rise'])}"
            f"{fmt(med(d['over']), 7)}{fmt(med(d['under']), 7)}"
        )
    print()
    print("closed loop (ms; shares)")
    print(f"{'driver':<16}{'class':<11}{'n':>4}{'lag50':>8}{'lag99':>8}{'late':>7}{'ans99':>8}{'slow99':>8}{'copies':>7}{'oracle':>7}{'probe':>7}")
    for (drv, g), d in sorted(loop.items(), key=lambda kv: (kv[0][1], kv[0][0])):
        orc = [copies_by.get((c, s, "oracle")) for c, s in d["runs"]]
        print(
            f"{drv:<16}{g:<11}{len(d['lag50']):>4}{fmt(med(d['lag50']), 8, 0)}{fmt(med(d['lag99']), 8, 0)}"
            f"{fmt(med(d['late']), 7, 3)}{fmt(med(d['ans99']), 8, 0)}{fmt(med(d['slow99']), 8, 0)}"
            f"{fmt(med(d['copies']), 7, 1)}{fmt(med(orc), 7, 1)}{fmt(med(d['probe']), 7, 3)}"
        )
    sens = defaultdict(list)
    for key, rs in pairs.items():
        if key[0] != "byzantine":
            continue
        clean = pairs.get(("byzclean",) + key[1:])
        if not clean:
            continue
        for m, r in rs.items():
            if (meters and m not in meters) or m not in clean:
                continue
            sens[m].append(sensitivity([x for x in r if x[0] >= warm], clean[m]))
    if sens:
        print()
        print("byzantine sensitivity (median |log2| of the reading with the liars over without, per run)")
        for m, xs in sorted(sens.items()):
            print(f"{m:<16}{len(xs):>4}  med {fmt(med(xs))}  max {fmt(max(x for x in xs if x is not None) if any(x is not None for x in xs) else None)}")


if __name__ == "__main__":
    main()
