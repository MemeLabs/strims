#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The testbed's twin: strims-sim's calibration_star run as a farm testbed
matrix ran the testbed, and the two compared.

    hack/testbed/twin.py run TAG --bin SCENARIOS [--seeds N] [--dir DIR]
                         KNOB=VALUE ...
    hack/testbed/twin.py lag MATRIX TAG ... --viewers N [--arms A,B]
    hack/testbed/twin.py swarm MATRIX TAG --viewers N [--arm A]
    hack/testbed/twin.py meter MATRIX TAG --viewers N [--arm A]
    hack/testbed/twin.py peers MATRIX TAG --viewers N [--arm A]
    hack/testbed/twin.py socks MATRIX [TAG] --viewers N [--arm A]

`run` runs calibration_star (the strims-sim scenarios test binary
SCENARIOS) with the window model (STRIMS_SIM_TCP=1) and the
STRIMS_SIM_CAL_<KNOB> settings given (a testbed matrix's env: VIEWERS,
TOPOLOGY, SEED_UP_STEPS, SECS, JOIN_AT, ...) at seeds 1..N (default 8),
eight at a time, and keeps each seed's series (calibration_series) as
DIR/TAG-<seed>.json (default target/twin); a seed already there is kept.

The others compare twin runs (DIR/TAG-*.json) with a farm testbed matrix's
runs (MATRIX, its rows.jsonl and runs/) of N viewers, in the phases of the
testbed's uplink cut (before 35-60 s, cut 60-180 s, after 180-238 s):

- `lag`: seconds whose viewers' p99 lag passes 1 s (the testbed: the worst
  viewer's second, a viewer without frames counted over), per run.
- `swarm`: the seed's copies, children, rate, pushes and pull answers, and
  the viewers' opens, prunes, switches, misses and pull requests a second.
- `meter`: the seed's capacity reading, queue delay, busy share, budget and
  copies.
- `peers`: the round trips and controller timeouts of the seed's links and
  the viewers'.
- `socks`: the testbed seed's TCP sockets (`ss`): round trips, unsent
  bytes and windows of the links that carried data and the rest; data
  segments in and out, pure acknowledgements and the tbf's drops, against
  the twin's drops and pure acknowledgements.
"""

import concurrent.futures as cf
import json
import os
import re
import statistics
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
PH = {"before": (35, 60), "cut": (60, 180), "after": (180, 238)}


def flag(args, name, default=None):
    if name in args:
        i = args.index(name)
        v = args[i + 1]
        del args[i : i + 2]
        return v
    return default


def q(xs, p):
    xs = sorted(x for x in xs if x is not None)
    return xs[int(p * (len(xs) - 1))] if xs else None


def mean(xs):
    xs = [x for x in xs if x is not None]
    return statistics.mean(xs) if xs else None


def f(v, w=7):
    return f"{'-':>{w}}" if v is None else f"{v:{w}.1f}"


# Inputs.

def testbed_runs(matrix, arm, n):
    """The run dirs of `arm` at `n` viewers that finished."""
    out = []
    for line in open(os.path.join(matrix, "rows.jsonl")):
        try:
            r = json.loads(line)
        except ValueError:
            continue  # a torn append's
        if r.get("exit") == 0 and r["arm"] == arm and int(r["env"]["VIEWERS"]) == n:
            out.append(os.path.join(matrix, r["run_dir"]))
    return out


def node_rows(d, i):
    return [json.loads(x) for x in open(os.path.join(d, f"node-{i}.jsonl"))]


def twin_runs(tdir, tag):
    out = []
    for seed in range(1, 1000):
        p = os.path.join(tdir, f"{tag}-{seed}.json")
        if not os.path.exists(p):
            if seed > 64:
                break
            continue
        out.append(json.load(open(p)))
    return out


# run

def cmd_run(args):
    binary = flag(args, "--bin")
    seeds = int(flag(args, "--seeds", "8"))
    tdir = flag(args, "--dir", os.path.join(ROOT, "target", "twin"))
    tag, knobs = args[0], args[1:]
    os.makedirs(tdir, exist_ok=True)

    def one(seed):
        path = os.path.join(tdir, f"{tag}-{seed}.json")
        if os.path.exists(path):
            return
        env = {**os.environ, "STRIMS_SIM_TCP": "1", "STRIMS_SIM_SEED": str(seed)}
        for kv in knobs:
            k, v = kv.split("=", 1)
            env[f"STRIMS_SIM_CAL_{k}"] = v
        p = subprocess.run([binary, "--exact", "calibration_star", "--include-ignored", "--nocapture",
                            "--test-threads", "1"], env=env, capture_output=True, text=True, check=False)
        at = p.stdout.find("SERIES {")
        if at < 0:
            print(f"seed {seed}: no series\n{p.stdout[-400:]}{p.stderr[-400:]}", file=sys.stderr)
            return
        with open(path, "w") as fh:
            fh.write(p.stdout[at + 7:].split("\n", 1)[0])

    with cf.ThreadPoolExecutor(8) as ex:
        list(ex.map(one, range(1, seeds + 1)))


# lag

def tb_over(d, n):
    lags = []
    for i in range(1, n + 1):
        lags.append({r["rel_ms"] // 1000: (r["lag_p99_ms"] if r["frames"] else None) for r in node_rows(d, i)})

    def over(lo, hi):
        return sum(1 for t in range(lo, hi) if any(lg.get(t) is None or lg[t] > 1000 for lg in lags))
    return over(35, 240), over(60, 180)


def tw_over(series):
    d = dict((int(a), b) for a, b in series["lag_p99"])

    def over(lo, hi):
        return sum(1 for t in range(lo, hi) if d.get(t, 0) > 1000 or d.get(t, 0) < 0)
    return over(35, 240), over(60, 180)


def cmd_lag(args):
    n = int(flag(args, "--viewers"))
    arms = flag(args, "--arms", "main,floor").split(",")
    tdir = flag(args, "--dir", os.path.join(ROOT, "target", "twin"))
    matrix, tags = args[0], args[1:]
    print(f"{n} viewers: seconds over 1 s at p99, 35-240 s (of 205) | cut 60-180 s (of 120), per run")
    rows = [(f"testbed {a}", [tb_over(d, n) for d in testbed_runs(matrix, a, n)]) for a in arms]
    rows += [(f"twin {t}", [tw_over(s) for s in twin_runs(tdir, t)]) for t in tags]
    for name, xs in rows:
        if xs:
            print(f"  {name:22} mean {mean([a for a, _ in xs]):5.1f} | {mean([b for _, b in xs]):5.1f}   "
                  + " ".join(f"{a}/{b}" for a, b in xs))


# swarm

def rate(series, key, lo, hi, peers_field=None):
    pts = []
    for s, t in series:
        if lo <= s <= hi:
            v = sum(p[peers_field] for p in t.get("peers", [])) if peers_field else t.get(key, 0)
            pts.append((s, v))
    if len(pts) < 2 or pts[-1][0] == pts[0][0]:
        return None
    return (pts[-1][1] - pts[0][1]) / (pts[-1][0] - pts[0][0])


def swarm(nodes, seed_rows):
    out = {}
    for ph, (lo, hi) in PH.items():
        sr = [r for s, r in seed_rows if lo <= s < hi]
        o = {"copies": mean(r["copies"] for r in sr), "children": mean(r["children"] for r in sr),
             "seed KB/s": mean(r["rate"] / 1e3 for r in sr)}
        seed = nodes[0]
        cs, pc = rate(seed, "chunks_sent", lo, hi), rate(seed, "pushed_chunks", lo, hi)
        o["seed pushes/s"] = pc
        o["seed answers/s"] = cs - pc if cs is not None and pc is not None else None
        o["seed requests in/s"] = rate(seed, None, lo, hi, "requests_in")
        for k in ("opens", "pruned", "switched", "misses", "late", "chunks_sent"):
            o[f"viewers {k}/s"] = sum(x for x in (rate(v, k, lo, hi) for v in nodes[1:]) if x is not None)
        o["viewers requests/s"] = sum(x for x in (rate(v, None, lo, hi, "requests_out") for v in nodes[1:])
                                      if x is not None)
        out[ph] = o
    return out


def tb_swarm(d, n):
    nodes = [[(r["rel_ms"] / 1000, r["transfers"]) for r in node_rows(d, i) if "transfers" in r] for i in range(n + 1)]
    seed = [(r["rel_ms"] / 1000, {"copies": r["seed_copies"], "children": r["children"], "rate": r["upload_rate"]})
            for r in node_rows(d, 0)]
    return swarm(nodes, seed)


def tw_swarm(series):
    return swarm([[(s, t) for s, t in h] for h in series["transfers"]], [(r["s"], r) for r in series["seed"]])


def cmd_swarm(args):
    n = int(flag(args, "--viewers"))
    arm = flag(args, "--arm", "floor")
    tdir = flag(args, "--dir", os.path.join(ROOT, "target", "twin"))
    matrix, tag = args[0], args[1]
    tb = [tb_swarm(d, n) for d in testbed_runs(matrix, arm, n)]
    tw = [tw_swarm(s) for s in twin_runs(tdir, tag)]
    for ph in PH:
        print(f"== {n} viewers, {ph}: testbed {arm} | twin {tag} (means over runs)")
        for k in tb[0][ph]:
            print(f"  {k:22} {f(mean(r[ph][k] for r in tb), 8)} | {f(mean(r[ph][k] for r in tw), 8)}")


# meter

def meter(rows):
    out = {}
    for ph, (lo, hi) in PH.items():
        r = [x for s, x in rows if lo <= s < hi]
        cap = [x["cap"] / 1e3 for x in r]
        out[ph] = {"capacity p10/50/90": (q(cap, .1), q(cap, .5), q(cap, .9)),
                   "queue delay p50/90": (q([x["qd"] for x in r], .5), q([x["qd"] for x in r], .9)),
                   "busy permille p50/90": (q([x["busy"] for x in r], .5), q([x["busy"] for x in r], .9)),
                   "budget p50": (q([x["budget"] for x in r], .5),),
                   "copies p50": (q([x["copies"] for x in r], .5),)}
    return out


def cmd_meter(args):
    n = int(flag(args, "--viewers"))
    arm = flag(args, "--arm", "floor")
    tdir = flag(args, "--dir", os.path.join(ROOT, "target", "twin"))
    matrix, tag = args[0], args[1]
    tb = [meter([(r["rel_ms"] / 1000, {"cap": r["capacity"], "qd": r["queue_delay_ms"], "busy": r["busy_permille"],
                                       "budget": r["seed_budget"], "copies": r["seed_copies"]})
                 for r in node_rows(d, 0)]) for d in testbed_runs(matrix, arm, n)]
    tw = [meter([(r["s"], {"cap": r["capacity"], "qd": r["qd"], "busy": r["busy"], "budget": r["budget"],
                           "copies": r["copies"]}) for r in s["seed"]]) for s in twin_runs(tdir, tag)]
    for ph in PH:
        print(f"== {n} viewers, {ph}: per run")
        for k in tb[0][ph]:
            fmt = lambda runs: "  ".join("/".join("-" if v is None else f"{v:.0f}" for v in r[ph][k]) for r in runs)
            print(f"  {k:21} testbed {fmt(tb)}")
            print(f"  {'':21} twin    {fmt(tw)}")


# peers

def peer_samples(nodes, lo, hi):
    out = {"seed rtt": [], "seed cto": [], "viewer rtt": [], "viewer cto": [], "viewer->seed rtt": []}
    for i, series in enumerate(nodes):
        for s, t in series:
            if not lo <= s < hi:
                continue
            for p in t.get("peers", []):
                if i == 0:
                    out["seed rtt"].append(p["rtt_ms"])
                    out["seed cto"].append(p["cto_ms"])
                elif p["peer"] == 0:
                    out["viewer->seed rtt"].append(p["rtt_ms"])
                else:
                    out["viewer rtt"].append(p["rtt_ms"])
                    out["viewer cto"].append(p["cto_ms"])
    return out


def cmd_peers(args):
    n = int(flag(args, "--viewers"))
    arm = flag(args, "--arm", "floor")
    tdir = flag(args, "--dir", os.path.join(ROOT, "target", "twin"))
    matrix, tag = args[0], args[1]
    lo, hi = PH["cut"]
    tb, tw = {}, {}
    for d in testbed_runs(matrix, arm, n):
        nodes = [[(r["rel_ms"] / 1000, r["transfers"]) for r in node_rows(d, i) if "transfers" in r]
                 for i in range(n + 1)]
        for k, v in peer_samples(nodes, lo, hi).items():
            tb.setdefault(k, []).extend(v)
    for s in twin_runs(tdir, tag):
        for k, v in peer_samples(s["transfers"], lo, hi).items():
            tw.setdefault(k, []).extend(v)
    qs = lambda xs: "/".join(f"{q(xs, p):.0f}" if xs else "-" for p in (.1, .5, .9, .99))
    print(f"{n} viewers, cut, ms p10/p50/p90/p99")
    for k in tb:
        print(f"  {k:18} testbed {qs(tb[k]):>20}   twin {qs(tw.get(k, [])):>20}")


# socks

def ss_blocks(d, offset):
    for b in open(os.path.join(d, "ss-0.log")).read().split("@ ")[1:]:
        lines = b.strip().split("\n")
        t = (int(lines[0]) - offset) / 1000
        for i in range(1, len(lines) - 1):
            m = re.search(r"10\.77\.0\.10:4000\s+10\.77\.0\.(\d+):|10\.77\.0\.(\d+):4000\s+10\.77\.0\.10:", lines[i])
            if m:
                yield t, int(m.group(1) or m.group(2)) - 10, lines[i + 1]


def ss_field(info, k):
    m = re.search(r"\b" + k + r":([0-9.]+)", info)
    return float(m.group(1)) if m else None


def cmd_socks(args):
    n = int(flag(args, "--viewers"))
    arm = flag(args, "--arm", "floor")
    tdir = flag(args, "--dir", os.path.join(ROOT, "target", "twin"))
    matrix, tags = args[0], args[1:]
    lo, hi = PH["cut"]
    agg, flows, drops = {}, [], []
    for d in testbed_runs(matrix, arm, n):
        seed = node_rows(d, 0)
        off = seed[0]["t_ms"] - seed[0]["rel_ms"]
        last, first, end = {}, {}, {}
        for t, peer, info in ss_blocks(d, off):
            sent = ss_field(info, "bytes_sent") or 0
            if not lo <= t < hi:
                last[peer] = sent
                continue
            cls = "data" if sent - last.get(peer, sent) > 30000 else "other"
            last[peer] = sent
            for k, unit in (("rtt", 1), ("notsent", 1024), ("cwnd", 1)):
                v = ss_field(info, k)
                if v is not None:
                    agg.setdefault((cls, k), []).append(v / unit)
            v = tuple(ss_field(info, k) or 0 for k in ("data_segs_in", "data_segs_out", "segs_out"))
            first.setdefault(peer, (t, v))
            end[peer] = (t, v)
        tot = [0.0, 0.0, 0.0]
        for p in first:
            (t0, a), (t1, b) = first[p], end[p]
            if t1 - t0 >= 60:
                tot[0] += (b[0] - a[0]) / (t1 - t0)
                tot[1] += (b[1] - a[1]) / (t1 - t0)
                tot[2] += ((b[2] - a[2]) - (b[1] - a[1])) / (t1 - t0)
        flows.append(tot)
        pat = re.compile(r"qdisc tbf 10:.*?dropped (\d+),")
        cut = sorted(int(pat.search(x).group(1)) for x in open(os.path.join(d, "tc-0.log"))
                     if pat.search(x) and lo * 1000 <= int(x.split()[0]) - off < hi * 1000)
        drops.append(cut[-1] - cut[0] if cut else 0)
    print(f"{n} viewers, the testbed seed's sockets in the cut ({arm}), p10/p50/p90/p99")
    for (cls, k), xs in sorted(agg.items()):
        unit = {"rtt": "ms", "notsent": "KiB", "cwnd": "pkts"}[k]
        print(f"  {cls:6} {k:8} {unit:4} " + "/".join(f"{q(xs, p):.0f}" for p in (.1, .5, .9, .99)))
    print(f"  per run: data segments in/s, data segments out/s, pure acks out/s: "
          + "  ".join(f"{a:.0f}/{b:.0f}/{c:.0f}" for a, b, c in flows))
    print(f"  tbf drops in the cut per run: {drops}")
    for t in tags:
        xs = twin_runs(tdir, t)
        if xs:
            secs = 240
            print(f"  twin {t}: drops over the run {[x.get('drops') for x in xs]}, "
                  f"pure acks/s {[round(x.get('acks', 0) / secs) for x in xs]}")


def main():
    cmds = {"run": cmd_run, "lag": cmd_lag, "swarm": cmd_swarm, "meter": cmd_meter, "peers": cmd_peers,
            "socks": cmd_socks}
    if len(sys.argv) < 2 or sys.argv[1] not in cmds:
        sys.exit(__doc__)
    cmds[sys.argv[1]](sys.argv[2:])


if __name__ == "__main__":
    main()
