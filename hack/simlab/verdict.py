# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The verdicts on the sims' runs, and the derived tables made from them.

The verdicts judge a run's series, `Report::series`'s fields
(hack/sim-db makes them from a run's tables, hack/capture/analyze.py
from a testbed run's nodes too), over the stream's seconds (not the
run's tail):

- Each viewer counts while it is there and past its own startup
  (`presence`): one there from the start from the stream's start, one
  that arrived later from its first frame plus JOIN_GRACE_SECS (its join
  plus that if it never reads), and either to its departure. Outside its
  span it is in no measure: neither the viewers present that trees count
  against, nor the lag, nor delivery. A series knows a viewer's stay by
  its `arrive_ms` and `depart_ms` (the viewers table's, from the stream's
  start: hack/sim-db's series carry them), else by strims-sim's -1 frames
  in the seconds it wasn't there; the ppspp sim's series alone (a viewer
  absent reads nothing) can't tell, and reads as one there throughout.
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
  LAG_SLOPE_FLOOR_MS (a spike that turns back sooner is no rise). The two
  are delivery's share of the latency budget (DELIVERY_MAX_MS and
  DELIVERY_TARGET_MS): Amazon IVS's 5 s maximum and 2 s target from the
  broadcaster's screen to the viewer's, less PIPELINE_MS, the stages the
  sims don't model.
- A verdict's onset is the first second of its run of bad seconds, or for
  delivery the first second whose window falls short.
- A collapsed run recovers once all three are healthy for RECOVER_SECS. It
  is absorbing when it hasn't recovered by the stream's end, or within
  ABSORB_SECS of the trigger going away (the seed's uplink stepping back
  up).
- A cell is feasible when the seed's lowest uplink carries one copy of the
  stream with OVERHEAD and the seed's spare plus the viewers' uplinks
  carry every viewer's copy: a collapse no tree could avoid isn't the
  protocol's. analyze.py's `map` marks cells feasible only at full
  uplink as "step".
- A run collapsed (sim-suite's scorecard, `collapsed`) when COLLAPSED_SECS
  seconds in a row are unhealthy and they last past its startup: the
  first STARTUP_SECS when viewers were there from the start (their own
  first seconds), none when every viewer arrived later (each arrival's
  are out of the measures already).

Over time, for hack/sim-db's tables and sim-suite's paired metrics:
`phases` names each second (startup, after an uplink step down or up, or
steady), `events` lists a run's (its start, the uplink's steps, viewers
coming and going, onset, recovery, collapses), `episodes` its healthy,
unhealthy and collapse spells, and `run_metrics` sums them up per run
(collapses, flaps, dwell times, onset after a drop, recovery after a
rise, excess lag, the lag's oscillation). VERDICT_VERSION hashes all of
it, so the tables made under another are stale. `viewer_phases` gives
each viewer's seconds their phase by the same rule, from `presence` and
`phases`: its own startup, then the second's.

The verdicts are stdlib alone: hack/capture/analyze.py re-exports their
names, and its capture tools run under the plain python3. The derived
tables are made from them, the cache's tables read through
store_host.duck (the DuckDB CLI, or the executor installed):

  series_of(meta, seconds, viewer_seconds, stays=None) -> dict | None
                 a run's series as its row's `series` has it, from its
                 tables
  load_series(pairs) -> {(src, k): (meta, series)}
                 the series of the runs `pairs` of (src, k), from the
                 cache
  judge(series) -> (verdict, run metrics) | (None, None)
  derived_version()
                 a hash of what defines the derived tables (DERIVED,
                 VERDICT_COLS, VERDICT_VERSION and the code that writes
                 them), their directories' `v=`
  derive_source(src)
                 the derived tables (verdicts, verdict_seconds, episodes,
                 derived_events, viewer_phases) of cache source `src`
                 where it lacks them, in this process, under a lock of
                 the source's own
  derive(sources, workers=None, progress=None) -> Derived
                 derive_source of each source, in a pool of `workers`
                 processes (half the cores by default, at most 8; 0 for
                 this one): concurrent.futures over multiprocessing's
                 "forkserver" context, whose server (a fresh interpreter,
                 this module preloaded, never duckdb) forks each worker,
                 so none inherits the caller's DuckDB instances, threads
                 or locks. Each worker opens one DuckDB connection, which
                 every source it judges reads and writes through (its
                 store_host.EXECUTOR); a pool takes RECYCLE sources a
                 worker, and the next pool's workers are new, so memory
                 can't creep. A source that fails comes back in
                 `failed`, its verdicts left for the next index;
                 `progress(message, done, total)` hears of each failure
                 and of every 50th source
"""

import concurrent.futures as cf
import collections
import dataclasses
import functools
import hashlib
import inspect
import json
import math
import multiprocessing
import os
import tempfile
from pathlib import Path

from . import store_host, vocab


# The latency budget: Amazon IVS's low-latency figures, glass to glass
# (an OBS broadcaster's screen to the viewer's), less what the sims don't
# model. Their lag runs from the seed's publish to the viewer's node
# holding the data.
E2E_TARGET_MS = 2000
E2E_MAX_MS = 5000
PIPELINE_MS = {
    "capture and encode": 150,  # a hardware encoder (NVENC, QuickSync, VideoToolbox)
    "ingest": 100,  # the RTMP leg; the ingest node writes units as they arrive
    "signing": 100,  # a unit's sign wait at 3 Mbit/s; the default cells publish whole writes
    "player": 333,  # the MSE player's margin over its p98 lateness, and the remuxer's frame
    "decode and display": 50,
}
# Delivery's share of the budget, rounded down to 50 ms: lag up to the
# target is the room an overhead trade may spend, and past the maximum a
# viewer is behind the product's worst case.
DELIVERY_TARGET_MS = (E2E_TARGET_MS - sum(PIPELINE_MS.values())) // 50 * 50
DELIVERY_MAX_MS = (E2E_MAX_MS - sum(PIPELINE_MS.values())) // 50 * 50

# The verdicts' thresholds (see the module docs).
TREES_SHARE = 0.5
TREES_SMOOTH = 3
TREES_SECS = 10
DELIVERY_SHARE = 0.95
DELIVERY_SECS = 10
LAG_QUANTILE = 0.9
LAG_HIGH_MS = DELIVERY_MAX_MS
LAG_HIGH_SECS = 5
LAG_SLOPE = 50
LAG_SLOPE_SECS = 20
LAG_SLOPE_FLOOR_MS = DELIVERY_TARGET_MS
RECOVER_SECS = 20
ABSORB_SECS = 60
# A run collapsed when this many seconds in a row are unhealthy and they
# last past its startup (the real stack's trees take about 10 s to form).
STARTUP_SECS = 15
COLLAPSED_SECS = 8
# A viewer that arrived after the start counts in the verdicts from its
# first frame plus this: an arrival's own startup, as the sims' rows leave
# its first 10 s out of its steady lag (the ppspp sim's SIM_WARM,
# strims-sim's JOIN_WARM).
JOIN_GRACE_SECS = 10
# How long after an uplink step its seconds belong to it (`phases`), and
# the window its onset and recovery are measured in (`run_metrics`) when
# no later step ends it first.
STEP_SECS = 60
# An oscillation: the lag series' autocorrelation peaks past this at some
# period of at least OSC_MIN_PERIOD seconds (`run_metrics`).
OSC_ACF = 0.3
OSC_MIN_PERIOD = 3
# The wire's bytes beyond the stream's, as a share of them, by chunk size:
# a lone viewer of a 100 Mbit seed in calibration_star (64 KiB munros at
# 1 KiB, 32 KiB at 4 KiB), its data, HAVEs, requests and integrity, with
# TCP's headers.
OVERHEAD = {1024: 0.34, 4096: 0.085}


def med(v):
    v = sorted(v)
    return v[len(v) // 2] if v else None


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


def presence(s):
    """Each viewer's stay, as the verdicts count it (see the module docs):
    (join, since, until), the first second `present` may count it in
    (None for one there from the start), and the seconds k it counts in,
    since <= k < until. One there from the start counts from 0, one that
    arrived later from its first frame's second plus JOIN_GRACE_SECS (its
    arrival's, if it never reads); either until its departure's second,
    which it left part-way through."""
    arrive, depart = s.get("arrive_ms") or [], s.get("depart_ms") or []
    out = []
    for i, frames in enumerate(s["frames"]):
        a = arrive[i] if i < len(arrive) else None
        d = depart[i] if i < len(depart) else None
        # strims-sim's frames are -1 in the seconds a viewer wasn't there.
        there = [k for k, f in enumerate(frames) if f >= 0]
        if a is None and there and frames[0] < 0:
            a = there[0] * 1000
        if d is None and there and frames[-1] < 0:
            d = there[-1] * 1000
        until = len(frames) if d is None else max(0, int(d // 1000))
        if a is None or a < 0:
            out.append((None, 0, until))
            continue
        # strims-sim samples an arrival first in the second it boots in,
        # the ppspp sim in the next: `present` may count it from either.
        join = int(a // 1000)
        first = next((k for k in range(join, len(frames)) if frames[k] > 0), join)
        out.append((join, first + JOIN_GRACE_SECS, until))
    return out


def startup(s):
    """The run's startup, the seconds a collapse must last past: STARTUP_SECS
    when an honest viewer was there from the start, else none (each
    arrival's own startup is out of the measures, `presence`)."""
    hostile = set(s.get("hostile", []))
    stays = [p for i, p in enumerate(presence(s)) if i not in hostile]
    return STARTUP_SECS if not stays or any(join is None for join, _, _ in stays) else 0


def measures(s):
    """The per-second measures the verdicts read, over the stream's seconds.
    A series whose stream varies in rate (a media run's) carries the chunks
    sent each second that `frames` counts (`written`), which on-time pushes
    count against, and is measured against those; older rows carry the
    chunks published (`chunks`), padding alone included. Delivery and lag
    leave out the viewers a series marks hostile (`hostile`), and every
    measure the seconds a viewer doesn't count in (`presence`): its hits
    are the series' sum over the viewers present, so an arrival's before it
    counts stay in it (the share then held to 1)."""
    n = min(s["secs"], len(s["present"]))
    per_s = s["rate"] / s["chunk"]
    chunks, written = s.get("chunks"), s.get("written")
    need = lambda k: chunks[k] if chunks else written[k] if written else per_s
    hostile = set(s.get("hostile", []))
    stays = presence(s)
    # The honest arrivals present (`present` counts them) that don't count
    # yet.
    early = [sum(1 for i, (join, since, until) in enumerate(stays)
                 if i not in hostile and join is not None and join <= k < min(since, until)) for k in range(n)]
    share = []
    for k in range(n):
        present = s["present"][k] - early[k]
        x = s["hits"][k] / (present * need(k)) if present > 0 and need(k) else None
        share.append(min(x, 1.0) if x is not None and early[k] else x)
    smooth = []
    for k in range(n):
        w = [x for x in share[max(0, k - TREES_SMOOTH // 2): k + TREES_SMOOTH // 2 + 1] if x is not None]
        smooth.append(sum(w) / len(w) if w else None)
    viewers = [(f, stays[i]) for i, f in enumerate(s["frames"]) if i not in hostile]
    firsts = [next((i for i, f in enumerate(frames) if f > 0), None) for frames, _ in viewers]

    def window(frames, k):
        got = sum(max(0, f) for f in frames[k - DELIVERY_SECS + 1: k + 1])
        if written:
            due = sum(written[k - DELIVERY_SECS + 1: k + 1])
            return got / due if due else None
        return got * s["frame"] / DELIVERY_SECS / s["rate"]

    deliv = []
    for k in range(n):
        d = [
            x for (frames, (_, since, until)), first in zip(viewers, firsts)
            if first is not None and k >= first + DELIVERY_SECS - 1 and since <= k < until
            for x in [window(frames, k)] if x is not None
        ]
        deliv.append(med(d))
    vl = [[x if since <= k < until else None for k, x in enumerate(v)]
          for i, (v, (_, since, until)) in enumerate(zip(lags(s), stays)) if i not in hostile]
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
        "unhealthy": 1 - sum(healthy) / n if n else None,
        # The seconds a collapse must last past (`startup`).
        "startup": startup(s),
        "lag_max": max((x for x in lag if x is not None), default=None),
        "healthy": healthy,
        # Second by second, what the verdicts read and which tests failed.
        "onset": onset, "hit_share": smooth, "delivery_share": deliv, "lag_q": lag,
        "trees_bad": trees_bad, "deliv_bad": deliv_bad, "lag_high": high, "lag_rise": rise,
    }


def run_startup(v):
    """A verdict's startup (`startup`): STARTUP_SECS for one made without."""
    return v.get("startup", STARTUP_SECS)


def collapsed(v):
    """Whether a run collapsed, by its verdict `v`: COLLAPSED_SECS unhealthy
    seconds in a row lasting past its startup (`startup`)."""
    run = 0
    first = run_startup(v)
    for k, h in enumerate(v["healthy"]):
        run = 0 if h else run + 1
        if run >= COLLAPSED_SECS and k >= first:
            return True
    return False


def steps(s):
    """The seed's uplink steps: (second, "seed_down" or "seed_up", the new
    uplink over the old)."""
    out = []
    for (t, u), (_, before) in zip(s["seed_up"][1:], s["seed_up"]):
        if u != before and before:
            out.append((t, "seed_down" if u < before else "seed_up", u / before))
    return out


def phases(s, v):
    """Each second's phase: `startup` (the run's, `startup`: none when every
    viewer arrived later), `after_down` or `after_up` (within STEP_SECS of
    the latest uplink step), else `steady`."""
    out = []
    st = steps(s)
    first = run_startup(v)
    for k in range(len(v["healthy"])):
        last = [(t, kind) for t, kind, _ in st if t <= k + 0.5 and k < t + STEP_SECS]
        if k < first:
            out.append("startup")
        elif last:
            out.append("after_down" if last[-1][1] == "seed_down" else "after_up")
        else:
            out.append("steady")
    return out


def viewer_phases(s, v):
    """Each viewer's phases, every viewer's (the hostile too): (viewer,
    phase, start, stop), its seconds start <= k < stop in one phase, in
    order. A viewer's seconds run from its join's second (`presence`'s
    join; 0 for one there from the start) to its departure's, which it was
    there part of (`presence`'s until), within the verdict's seconds: in
    `startup` until its own startup ends (`presence`'s since), then in the
    second's phase (`phases`). `viewer` is the series' `viewer` id, else
    its place in the series."""
    ph = phases(s, v)
    ids = s.get("viewer") or range(len(s["frames"]))
    out = []
    for i, (join, since, until) in zip(ids, presence(s)):
        for k in range(join or 0, min(len(ph), until + 1)):
            p = "startup" if k < since else ph[k]
            if out and out[-1][0] == i and out[-1][1] == p and out[-1][3] == k:
                out[-1][3] = k + 1
            else:
                out.append([i, p, k, k + 1])
    return [tuple(x) for x in out]


def events(s, v):
    """A run's events, (second from the stream's start, kind, value): its
    `start`; the seed's uplink steps (`seed_down`, `seed_up`, the new
    uplink over the old); the viewers present rising and falling
    (`present_join`, `present_leave`, by how many, from `present`; the
    sims' own `join` and `leave` are a node's); the verdict's `onset` and
    `recovered`; and each `collapse` (`episodes`), with its length."""
    out = [(0, "start", None)]
    out += [(t, kind, ratio) for t, kind, ratio in steps(s)]
    present = s["present"][: len(v["healthy"])]
    for k in range(1, len(present)):
        d = present[k] - present[k - 1]
        if d:
            out.append((k, "present_join" if d > 0 else "present_leave", abs(d)))
    if v["onset"] is not None:
        out.append((v["onset"], "onset", None))
    if v["recovered"] is not None:
        out.append((v["recovered"], "recovered", None))
    out += [(a, "collapse", b - a) for kind, a, b in episodes(v) if kind == "collapse"]
    return sorted(out, key=lambda e: (e[0], e[1]))


def episodes(v):
    """A run's spells, (kind, first second, the second after the last):
    every run of `healthy` and of `unhealthy` seconds, and the unhealthy
    ones that make a collapse (`collapsed`'s rule) again as `collapse`."""
    out = []
    h = v["healthy"]
    a = 0
    first = run_startup(v)
    for k in range(1, len(h) + 1):
        if k == len(h) or h[k] != h[a]:
            kind = "healthy" if h[a] else "unhealthy"
            out.append((kind, a, k))
            if not h[a] and k - a >= COLLAPSED_SECS and k - 1 >= first:
                out.append(("collapse", a, k))
            a = k
    return out


def acf(x, lag):
    """`x`'s autocorrelation at `lag`."""
    n = len(x)
    m = sum(x) / n
    den = sum((y - m) ** 2 for y in x)
    if not den or lag >= n:
        return None
    return sum((x[i] - m) * (x[i + lag] - m) for i in range(n - lag)) / den


def run_metrics(s, v):
    """A run's dynamics, from its series `s` and verdict `v`, as scalars
    that pair by seed:

      collapses               collapse spells (`episodes`)
      flaps_per_min           changes between healthy and unhealthy seconds
                              after the run's startup (`startup`), a minute
      healthy_dwell_p50_s     ... and p90: how long healthy spells last
      unhealthy_dwell_p50_s   ... and p90: unhealthy ones
      onset_after_drop_s      seconds from the first uplink step down to
                              its first unhealthy second, at most the
                              window (the next step, STEP_SECS, or the
                              stream's end), `_censored` when none came
      recover_after_rise_s    seconds from the first step up after a step
                              down to a healthy spell of RECOVER_SECS, at
                              most the stream's end (`_censored`)
      excess_lag_area_ms_s    the lag past DELIVERY_TARGET_MS summed over
                              seconds (ms x s); `excess_lag_peak_ms` its
                              largest
      osc_amp_ms              half the p10-p90 spread of the lag after
                              the startup; `osc_period_s` and `osc_acf`
                              the autocorrelation's highest peak past
                              OSC_ACF at a period of OSC_MIN_PERIOD or
                              more, when one is"""
    h = v["healthy"]
    n = len(h)
    eps = episodes(v)
    spells = lambda kind: [b - a for k, a, b in eps if k == kind]
    first = run_startup(v)
    after = h[first:]
    flaps = sum(1 for a, b in zip(after, after[1:]) if a != b)
    out = {
        "collapses": len(spells("collapse")),
        "flaps_per_min": flaps * 60 / len(after) if after else None,
        "healthy_dwell_p50_s": quantile(spells("healthy"), 0.5),
        "healthy_dwell_p90_s": quantile(spells("healthy"), 0.9),
        "unhealthy_dwell_p50_s": quantile(spells("unhealthy"), 0.5),
        "unhealthy_dwell_p90_s": quantile(spells("unhealthy"), 0.9),
    }
    st = steps(s)
    down = next(((i, t) for i, (t, kind, _) in enumerate(st) if kind == "seed_down"), None)
    out["onset_after_drop_s"] = out["onset_after_drop_censored"] = None
    out["recover_after_rise_s"] = out["recover_after_rise_censored"] = None
    if down is not None:
        i, t = down
        end = min(n, t + STEP_SECS, *(t2 for t2, _, _ in st[i + 1:]))
        k0 = int(math.ceil(t - 0.5))
        bad = next((k for k in range(max(0, k0), int(end)) if not h[k]), None)
        out["onset_after_drop_s"] = (bad if bad is not None else end) - t
        out["onset_after_drop_censored"] = bad is None
        up = next((t2 for t2, kind, _ in st[i + 1:] if kind == "seed_up"), None)
        if up is not None:
            k1 = max(0, int(math.ceil(up - 0.5)))
            ok = next((k for k in range(k1, n - RECOVER_SECS + 1) if all(h[k: k + RECOVER_SECS])), None)
            out["recover_after_rise_s"] = (ok if ok is not None else n) - up
            out["recover_after_rise_censored"] = ok is None
    lag = v["lag_q"]
    excess = [max(0, x - DELIVERY_TARGET_MS) for x in lag if x is not None]
    out["excess_lag_area_ms_s"] = sum(excess) if excess else None
    out["excess_lag_peak_ms"] = max(excess) if excess else None
    tail = [x for x in lag[first:] if x is not None]
    out["osc_amp_ms"] = out["osc_period_s"] = out["osc_acf"] = None
    if len(tail) >= 4 * OSC_MIN_PERIOD:
        out["osc_amp_ms"] = (quantile(tail, 0.9) - quantile(tail, 0.1)) / 2
        best = None
        r = [acf(tail, p) for p in range(len(tail) // 2 + 2)]
        for p in range(OSC_MIN_PERIOD, len(tail) // 2 + 1):
            if r[p] is None or r[p] < OSC_ACF or (r[p - 1] or 0) > r[p] or (r[p + 1] or 0) > r[p]:
                continue
            if best is None or r[p] > best[1]:
                best = (p, r[p])
        if best:
            out["osc_period_s"], out["osc_acf"] = best
    return out


def verdict_version():
    """A hash of everything that defines the verdicts and the run metrics:
    a derived table made under another is stale."""
    import inspect

    parts = [inspect.getsource(f) for f in (lags, presence, startup, measures, slope, runs_of, quantile, med,
                                            verdict, feasible, run_startup, collapsed, steps, phases, events,
                                            episodes, acf, run_metrics)]
    parts.append(repr([TREES_SHARE, TREES_SMOOTH, TREES_SECS, DELIVERY_SHARE, DELIVERY_SECS, LAG_QUANTILE,
                       LAG_HIGH_MS, LAG_HIGH_SECS, LAG_SLOPE, LAG_SLOPE_SECS, LAG_SLOPE_FLOOR_MS, RECOVER_SECS,
                       ABSORB_SECS, STARTUP_SECS, COLLAPSED_SECS, JOIN_GRACE_SECS, STEP_SECS, OSC_ACF,
                       OSC_MIN_PERIOD, DELIVERY_TARGET_MS, OVERHEAD]))
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()[:12]



def feasible(s):
    """True, False, or "step": feasible at the seed's full uplink only."""
    copy = s["rate"] * (1 + OVERHEAD.get(s["chunk"], OVERHEAD[1024]))
    viewers = len(s["lag"])
    ok = lambda up: up >= copy and up + s["viewer_up"] >= viewers * copy
    lo, hi = min(u for _, u in s["seed_up"]), s["seed_up"][0][1]
    return True if ok(lo) else ("step" if ok(hi) else False)


# Derived tables (hack/sim-db) are made under this.
VERDICT_VERSION = verdict_version()


# --- the derived tables --------------------------------------------------------------

# series_meta.json's keys that aren't the series'.
META_ONLY = {"sim", "seconds", "viewers", "bin_ms"}


def files(srcs, table):
    """`table`'s Parquet files in the cache's sources `srcs`."""
    root = store_host.cache_root() / "cache"
    return [str(root / s / f"{table}.parquet") for s in srcs if (root / s / f"{table}.parquet").exists()]


def plist(paths):
    return "[" + ", ".join(store_host.q(p) for p in paths) + "]"


def series_of(meta, seconds, viewer_seconds, stays=None):
    """A run's series as its row's `series` has it, from its tables: the
    meta's fields, each `seconds` column (a list by second), and per viewer
    in the order of their ids `viewer` (its id), `lag` and `frames`; with
    `stays` (the viewers table's rows: viewer, arrive_ms, depart_ms), per
    viewer `arrive_ms` and `depart_ms`, the stays presence reads (None for
    a departure that didn't come)."""
    if not seconds:
        return None
    # A row's series leaves `hostile` out without any.
    s = {k: v for k, v in meta.items() if k not in META_ONLY and not (k == "hostile" and not v)}
    seconds = sorted(seconds, key=lambda r: r["sec"])
    for c in seconds[0]:
        if c in ("k", "src", "sec"):
            continue
        v = [r.get(c) for r in seconds]
        if any(x is not None for x in v):
            s[c] = v
    by = sorted(viewer_seconds, key=lambda r: r["viewer"])
    # strims-sim writes no viewer_seconds row for a second its viewer
    # wasn't there in: the series' -1 frames (and lag) there.
    n = len(seconds)
    def by_sec(r, f):
        out = [-1] * n
        for k, x in zip(r.get("sec") or range(len(r[f])), r[f]):
            if 0 <= k < n:
                out[k] = x
        return out
    s["viewer"] = [r["viewer"] for r in by]
    s["lag"] = [by_sec(r, "lag") for r in by]
    s["frames"] = [by_sec(r, "frames") for r in by]
    if stays:
        at = {r["viewer"]: r for r in stays}
        if all(r["viewer"] in at for r in by):
            s["arrive_ms"] = [at[r["viewer"]]["arrive_ms"] for r in by]
            s["depart_ms"] = [at[r["viewer"]]["depart_ms"] for r in by]
    return s


def wanted(pairs):
    """A VALUES list of (src, k) for a join."""
    q = store_host.q
    return "(VALUES " + ", ".join(f"({q(s)}, {q(k)})" for s, k in pairs) + ") w(src, k)"


def load_series(pairs):
    """{(src, k): (meta, series)} for `pairs` of (src, k), from the cache."""
    duck = store_host.duck
    pairs = sorted(set(pairs))
    srcs = sorted({s for s, _ in pairs})
    runs = files(srcs, "runs")
    if not pairs or not runs:
        return {}
    want = wanted(pairs)
    meta = {(r["src"], r["k"]): json.loads(r["meta"] or "{}") for r in duck(
        f"SELECT src, k, meta FROM read_parquet({plist(runs)}, union_by_name=true) SEMI JOIN {want} USING (src, k);")}
    secs = collections.defaultdict(list)
    if files(srcs, "seconds"):
        for r in duck(f"SELECT * FROM read_parquet({plist(files(srcs, 'seconds'))}, union_by_name=true) "
                      f"SEMI JOIN {want} USING (src, k);"):
            secs[(r["src"], r["k"])].append(r)
    vs = collections.defaultdict(list)
    if files(srcs, "viewer_seconds"):
        for r in duck(f"SELECT src, k, viewer, list(sec ORDER BY sec) AS sec, list(lag_ms ORDER BY sec) AS lag, "
                      f"list(frames ORDER BY sec) AS frames "
                      f"FROM read_parquet({plist(files(srcs, 'viewer_seconds'))}, union_by_name=true) "
                      f"SEMI JOIN {want} USING (src, k) GROUP BY ALL;"):
            vs[(r["src"], r["k"])].append(r)
    # The viewers' stays, where the tables have them (both sims' since
    # their arrivals and departures).
    stays = collections.defaultdict(list)
    viewers = files(srcs, "viewers")
    if viewers:
        table = f"read_parquet({plist(viewers)}, union_by_name=true)"
        cols = {r["column_name"] for r in duck(f"DESCRIBE SELECT * FROM {table};")}
        if {"arrive_ms", "depart_ms"} <= cols:
            for r in duck(f"SELECT src, k, viewer, arrive_ms, depart_ms FROM {table} "
                          f"SEMI JOIN {want} USING (src, k);"):
                stays[(r["src"], r["k"])].append(r)
    return {p: (m, series_of(m, secs.get(p, []), vs.get(p, []), stays.get(p))) for p, m in meta.items()}


def judge(s):
    """A series' verdict and run metrics, or (None, None)."""
    if not s:
        return None, None
    try:
        v = verdict(s)
    except Exception:
        return None, None
    return v, run_metrics(s, v)


DERIVED = {
    "verdict_seconds": {"k": "VARCHAR", "src": "VARCHAR", "sec": "BIGINT", "phase": "VARCHAR", "healthy": "BOOLEAN",
                        "trees_bad": "BOOLEAN", "deliv_bad": "BOOLEAN", "lag_high": "BOOLEAN", "lag_rise": "BOOLEAN",
                        "hit_share": "DOUBLE", "delivery": "DOUBLE", "lag_q": "DOUBLE"},
    "episodes": {"k": "VARCHAR", "src": "VARCHAR", "kind": "VARCHAR", "start": "BIGINT", "stop": "BIGINT",
                 "len": "BIGINT"},
    "derived_events": {"k": "VARCHAR", "src": "VARCHAR", "sec": "DOUBLE", "kind": "VARCHAR", "value": "DOUBLE"},
    "viewer_phases": {"k": "VARCHAR", "src": "VARCHAR", "viewer": "BIGINT", "phase": "VARCHAR", "start": "BIGINT",
                      "stop": "BIGINT"},
}
# derived_events' kinds (events'): the vocabulary's derived ones, in its
# order, none of them the sims'.
DERIVED_KINDS = tuple(vocab.derived())
# A run's verdict, then run_metrics' columns. startup_s is the seconds a
# collapse must last past (startup).
VERDICT_COLS = {"k": "VARCHAR", "src": "VARCHAR", "onset": "BIGINT", "trees": "BIGINT", "delivery": "BIGINT",
                "latency": "BIGINT", "recovered": "BIGINT", "absorbing": "BOOLEAN", "feasible": "VARCHAR",
                "unhealthy": "DOUBLE", "lag_p50": "DOUBLE", "lag_max": "DOUBLE", "collapsed": "BOOLEAN",
                "startup_s": "BIGINT"}


@functools.cache
def derived_version():
    """A hash of everything that defines the derived tables:
    VERDICT_VERSION, their columns and the code that writes them
    (DERIVE_CODE), so a derived table made under another is stale."""
    parts = [VERDICT_VERSION, repr([DERIVED, DERIVED_KINDS, VERDICT_COLS]), *DERIVE_SOURCES]
    return hashlib.sha256("\n".join(parts).encode()).hexdigest()[:12]


def derived_dir(table):
    """Derived `table`'s files: a source's each, under derived_version."""
    return store_host.cache_root() / "derived" / table / f"v={derived_version()}"


def derive_source(src):
    """Makes the derived tables of cache source `src` where it lacks them
    under the current derived_version, under a lock of its own
    (derived/locks/), so processes deriving other sources don't wait."""
    name = src.replace("/", "~") + ".parquet"
    if all((derived_dir(t) / name).exists() for t in [*DERIVED, "verdicts"]):
        return
    with store_host.locked(store_host.cache_root() / "derived" / "locks" / f"{name}.lock"):
        if all((derived_dir(t) / name).exists() for t in [*DERIVED, "verdicts"]):
            return
        keys = [r["k"] for r in store_host.duck(f"SELECT k FROM read_parquet({plist(files([src], 'runs'))});")]
        got = load_series([(src, k) for k in keys])
        out = {t: [] for t in DERIVED}
        verdicts = []
        metric_names = None
        for (_, k), (meta, s) in got.items():
            v, m = judge(s)
            if v is None:
                continue
            if metric_names is None:
                metric_names = list(m)
            ph = phases(s, v)
            for sec in range(len(v["healthy"])):
                out["verdict_seconds"].append([k, src, sec, ph[sec], v["healthy"][sec], v["trees_bad"][sec],
                                               v["deliv_bad"][sec], v["lag_high"][sec], v["lag_rise"][sec],
                                               v["hit_share"][sec], v["delivery_share"][sec], v["lag_q"][sec]])
            out["episodes"] += [[k, src, kind, x, y, y - x] for kind, x, y in episodes(v)]
            out["derived_events"] += [[k, src, t, kind, value] for t, kind, value in events(s, v)]
            out["viewer_phases"] += [[k, src, *span] for span in viewer_phases(s, v)]
            verdicts.append([k, src, v["onset"], v["trees"], v["delivery"], v["latency"], v["recovered"],
                             v["absorbing"], str(v["feasible"]), v["unhealthy"], v["lag_p50"], v["lag_max"],
                             collapsed(v), run_startup(v)] + [m[n] for n in metric_names])
        cols = dict(VERDICT_COLS)
        for n in metric_names or []:
            cols[n] = "BOOLEAN" if n.endswith("_censored") else "DOUBLE"
        write_table(derived_dir("verdicts") / name, cols, verdicts)
        for t, c in DERIVED.items():
            write_table(derived_dir(t) / name, c, out[t])


def write_table(path, cols, data):
    """Writes rows `data` (lists in `cols`' order) to Parquet at `path`,
    its columns `cols` alone: the CSV it's read from sits in the derived
    dir's `v=` directory, which DuckDB would take for a hive partition and
    add as a column."""
    path.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile("w", suffix=".csv", dir=path.parent, delete=False) as f:
        f.write(",".join(cols) + "\n")
        for r in data:
            f.write(",".join("" if x is None else store_host.csv_text(str(x).lower() if isinstance(x, bool) else x)
                             for x in r) + "\n")
        tmp = Path(f.name)
    part = path.with_suffix(f".tmp{os.getpid()}")
    q = store_host.q
    try:
        store_host.duck(f"COPY (SELECT * FROM read_csv({q(tmp)}, header=true, auto_detect=false, "
                        f"columns={store_host.struct(cols)}, nullstr='', hive_partitioning=false)) TO {q(part)} "
                        "(FORMAT parquet, COMPRESSION zstd);", out=None)
        part.replace(path)
    finally:
        tmp.unlink(missing_ok=True)
        part.unlink(missing_ok=True)


# The code that writes the derived tables, whose source derived_version
# hashes: read here, as the module loads, so a process hashes the code it
# runs. inspect.getsource reads the file, so a later call in a process
# that outlives an edit (the console server's forkserver children, which
# fork from the code they started with) would hash the edited file's text
# against the loaded code and name a version no derive writes.
DERIVE_CODE = (derive_source, write_table, viewer_phases)
DERIVE_SOURCES = tuple(inspect.getsource(f) for f in DERIVE_CODE)


# --- a pool of derives ----------------------------------------------------------------

# A pool's sources a worker, each pool's workers new.
RECYCLE = 50
# A worker's DuckDB connection: its threads and memory limit.
WORKER_THREADS = 2
WORKER_MEMORY = "2GB"


@dataclasses.dataclass
class Derived:
    """What a derive made: the sources it judged, and those it couldn't,
    {src: error}."""
    judged: list = dataclasses.field(default_factory=list)
    failed: dict = dataclasses.field(default_factory=dict)


def default_workers():
    return max(1, min(8, (os.cpu_count() or 2) // 2))


def derive(sources, workers=None, progress=None):
    """derive_source of each of `sources` (the module docs): `workers` at a
    time in a forkserver pool, or in this process with 0."""
    say = progress or (lambda message, done, total: None)
    srcs = list(dict.fromkeys(sources))
    out = Derived()
    total = len(srcs)

    def record(src, err, done):
        if err is None:
            out.judged.append(src)
        else:
            out.failed[src] = err
            say(f"verdicts: {src}: {err}", done, total)
        if done % 50 == 0 or done == total:
            say(f"verdicts: {done}/{total} sources", done, total)

    if not srcs:
        return out
    if workers == 0:
        for n, src in enumerate(srcs, 1):
            record(src, judged(src), n)
        return out
    ctx = multiprocessing.get_context("forkserver")
    ctx.set_forkserver_preload([__name__])
    n = max(1, min(workers or default_workers(), total))
    done = 0
    # A pool takes RECYCLE sources a worker, and its workers end with it:
    # the executor's own max_tasks_per_child leaves a worker that ends
    # unreplaced once workers have been idle (its idle count only grows),
    # and the pool hangs when the last one ends.
    for i in range(0, total, n * RECYCLE):
        part = srcs[i:i + n * RECYCLE]
        with cf.ProcessPoolExecutor(min(n, len(part)), mp_context=ctx, initializer=start_worker,
                                    initargs=(str(store_host.cache_root()),)) as pool:
            futures = {pool.submit(judged, src): src for src in part}
            for f in cf.as_completed(futures):
                try:
                    err = f.result()
                except cf.BrokenExecutor as e:
                    # A worker that died took the sources it held with it.
                    err = f"its worker died: {e}"
                done += 1
                record(futures[f], err, done)
    return out


def judged(src):
    """derive_source of `src`: None, or why it failed."""
    try:
        derive_source(src)
    except Exception as e:
        return str(e) or type(e).__name__
    return None


def start_worker(root):
    """A derive worker's start: the caller's cache root (the forkserver's
    environment is the one it started with), and its one DuckDB
    connection, every store_host.duck of the sources it judges run on
    it."""
    from . import engine, store

    os.environ["SIM_DB_ROOT"] = root
    tmp = Path(root) / "tmp"
    if tmp.parent.exists():
        tmp.mkdir(exist_ok=True)
    store_host.EXECUTOR = store.executor(engine.store(":memory:", WORKER_THREADS, WORKER_MEMORY,
                                                     tmp if tmp.exists() else None))
