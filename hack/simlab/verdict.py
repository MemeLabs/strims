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
it, so the tables made under another are stale.

The verdicts are stdlib alone: hack/capture/analyze.py, which
re-exports their names, and hack/sim-db/simdb.py run under the plain
python3. The derived tables are made from them:

  series_of(meta, seconds, viewer_seconds, stays=None) -> dict | None
                 simdb.series_of: a run's series from its tables
  load_series(pairs) -> {(src, k): (meta, series)}
                 simdb.load_series, over the cache's sources
  derive(sources, workers=None, progress=None) -> Derived
                 the derived tables (verdicts, verdict_seconds, episodes,
                 derived_events) of each source lacking them, judged in a
                 forkserver pool (concurrent.futures over
                 multiprocessing's "forkserver" context, the pure verdict
                 code preloaded, never duckdb), each worker with one
                 DuckDB connection of its own, recycled after 50 sources;
                 a source that fails comes back in `failed`, its verdicts
                 left for the next index
"""

import dataclasses
import hashlib
import math


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


@dataclasses.dataclass
class Derived:
    """What a derive made: the sources it judged, and those it couldn't,
    {src: error}."""
    judged: list = dataclasses.field(default_factory=list)
    failed: dict = dataclasses.field(default_factory=dict)


def series_of(meta, seconds, viewer_seconds, stays=None):
    raise NotImplementedError("simlab.verdict.series_of: increment I3")


def load_series(pairs):
    raise NotImplementedError("simlab.verdict.load_series: increment I3")


def derive(sources, workers=None, progress=None):
    raise NotImplementedError("simlab.verdict.derive: increment I3")
