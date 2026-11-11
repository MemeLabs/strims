# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The arrival forecast study's regime description: a hidden semi-Markov
model (HSMM) of each peer's availability as a source of each stream, fit
offline over the catalog's runs (simlab.forecast's tables), in pure
Python on a deterministic sample of sequences.

  sequences(conn, sample=SAMPLE) -> [Sequence]
  fit(seqs, k=K, iters=ITERS, d=D, seed=0, say=None) -> Model
  forward_backward(model, seq), decode(model, seq) -> [state]
  regimes(sel, conn=None, sample=SAMPLE, decode_sample=DECODE_SAMPLE,
          iters=ITERS, say=None) -> ({table: [dict]}, Model)
  regimes_text(tables) -> str

The description's tables: states (each state's emissions, fitted
durations and transitions), occupancy, dwell and transitions of the
decoded paths by arm and cell, events (the states around events),
swarm (the shared gap and what correlation between peers it leaves),
stability (the fit on odd seeds and on even ones beside the pooled fit)
and seed_spread (each state's share over seeds), availability (the
decoded state as a predictor of a request's failure, forecast's
explained scoring) and by_receiver (forecast.agreement_sql).

A sequence is a (run, peer, stream)'s, one observation a second of the
peer's holds of the stream's chunks published in that second
(forecast.prepare's fs_arr and fs_pub): its worst lag class (LAG_EDGES_MS:
the hold less the chunk's publication; a chunk never held the worst
class), whether it pulled any of them (an answer to its request, off its
push path), and the class's trend against the last second's. A second
without a chunk of the stream has no observation. A peer's sequence runs
from JOIN_GRACE_S after it arrived to its departure. The truth of a
peer's holds is every receiver's: a receiver sees the same peer state, its
link adding half a round trip (tens of ms, below the first lag edge), so
the per-(receiver, peer, stream) sequences the study names are a (peer,
stream) sequence each; the description checks what is left by receiver
in the decisions' failures (`by_receiver`).

The grid is 1 s: four chunks of a stream (a stream signs one every 250 ms
at the cells' rate), four STATE intervals (STATE_EVERY, 250 ms), the
verdict's seconds and viewer_phases'; the regimes in question (an
overload, a break and its recovery, a parent's loss) last seconds.

The shared swarm state of a stream is the seed's push gap: a second some
chunk of the stream published in stayed seed-only past GAP_MS (no other
node held it by then). It is observed, per (run, stream, second), and
taken as a covariate: each peer state's emissions are conditioned on it,
so a gap the whole swarm shares doesn't make a per-peer state. What
correlation between peers is left once the gap is accounted for is the
swarm table's overdispersion: the variance across a (run, stream)'s
seconds of how many of its peers are in a bad state, against the
binomial variance of independent peers with the same per-second rate, by
swarm state (1 for independent peers).

The model: K states, each with a categorical emission per observation
component given the swarm state (naive Bayes), an explicit duration
distribution over 1..D seconds and transitions between states, a state
allowed to follow itself (so a dwell past D seconds is a run of segments:
dwell times are read from the decoded paths). EM (forward-backward over
segments, in log space), the first segment taken as complete and the last
as right-censored (its duration's survival); emissions and durations get
half a count of prior mass each (Jeffreys), so an empty cell keeps a
probability. States are named after the fit by their emissions out of
a gap (`names`' rules), by the two things that tell them apart in the
runs: whether the peer pulls the stream's chunks (off its push path) and
how late it holds them: on_path_fresh, on_path_late, pulled_fresh,
pulled_late. The plan's candidates map onto them as on-path steady,
overloaded (on path and late: its lag's trend shows no rise of its own),
off-path with holes (pulled; out of a gap fresh, in one missing) and
recovering (no state of its own: a recovery passes through on_path_late
back to on_path_fresh). A name is a label for the reader, not a claim:
the states table gives each state's emissions.
"""

import dataclasses
import math
import random
from pathlib import Path

# The grid, the join grace, the lag classes' edges (main's hold lag p99 in
# the steady cells is about the first; three times it; past the delivery
# target and push's candidate freshness), and the swarm gap.
GRID_MS = 1000
JOIN_GRACE_S = 2
LAG_EDGES_MS = (250, 750, 2500)
GAP_MS = 250
# The model's size, EM's iterations and the longest explicit duration
# (a dwell past it is a run of segments, the state following itself).
K = 4
ITERS = 10
D = 15
# The fit reads every FIT_THIN-th of its sampled sequences (pure Python:
# a fit's cost is its seconds times K times D).
FIT_THIN = 2
# Streams sampled a run, by hash, each with every peer's sequence: for the
# fit, and for the decoded description (whose swarm check needs a stream's
# every peer).
SAMPLE = 1
DECODE_SAMPLE = 2
# The observation's components and their sizes: lag class, pulled, trend.
COMPONENTS = (("lag", len(LAG_EDGES_MS) + 1), ("pulled", 2), ("trend", 3))
PRIOR = 0.5
NEG = -1e300


@dataclasses.dataclass
class Sequence:
    """A (run, peer, stream)'s observations: `obs` a tuple per second
    (lag class, pulled, trend; None for none), `swarm` the stream's gap
    flag per second, `phase` the peer's phase per second, `t0` the first
    second, and its run's labels."""
    k: str
    arm: str
    cell: str
    seed: int
    peer: int
    stream: int
    t0: int
    obs: list
    swarm: list
    phase: list


def lag_class(lag_ms):
    """A hold lag's class (LAG_EDGES_MS), the worst for none."""
    if lag_ms is None:
        return len(LAG_EDGES_MS)
    for i, e in enumerate(LAG_EDGES_MS):
        if lag_ms <= e:
            return i
    return len(LAG_EDGES_MS)


def observations_sql(sample, grid_ms=GRID_MS):
    """SQL over forecast.prepare's tables: the steps (`grid_ms` long) of
    every peer's sequence of `sample` streams a run (by hash of (k,
    stream); every stream for None), each step's worst lag, whether the
    peer pulled any of its chunks, the stream's swarm gap and the peer's
    phase."""
    keep = "true" if sample is None else f"r <= {int(sample)}"
    return f"""
WITH n AS (
    -- A run's streams: every node opens a parent for each (a chunk's
    -- stream is its number modulo their count).
    SELECT p.k, count(DISTINCT p.stream) AS n FROM parents p JOIN fs_jobs j USING (k, src) GROUP BY p.k
), chunks AS (
    SELECT p.k, p.seq, p.pub_ms, p.first_ms, p.seq % n.n AS stream, p.pub_ms // {grid_ms} AS sec
    FROM fs_pub p JOIN n USING (k) WHERE p.pub_ms IS NOT NULL
), swarm AS (
    SELECT k, stream, sec, bool_or(first_ms IS NULL OR first_ms - pub_ms > {GAP_MS}) AS gap
    FROM chunks GROUP BY ALL
), streams AS (
    SELECT k, stream, row_number() OVER (PARTITION BY k ORDER BY hash(k, stream)) AS r
    FROM (SELECT DISTINCT k, stream FROM chunks)
), peers AS (
    SELECT v.k, v.node, coalesce(greatest(v.arrive_ms, 0), 0) // {grid_ms} + {JOIN_GRACE_S * 1000 // grid_ms} AS s0,
           coalesce(v.depart_ms, j.end_ms) // {grid_ms} AS s1, j.arm, j.cell, j.seed
    FROM viewers v JOIN fs_jobs j USING (k, src) WHERE NOT coalesce(v.hostile, false)
), kept AS (
    SELECT p.*, s.stream FROM peers p JOIN streams s USING (k) WHERE {keep}
), firsts AS (
    SELECT k, node, seq, min(held_ms) AS held_ms, arg_min(how, held_ms) AS how FROM fs_holds GROUP BY ALL
)
SELECT q.arm, q.cell, q.seed, q.k, q.node AS peer, q.stream, c.sec,
       max(coalesce(h.held_ms - c.pub_ms, 1e9)) AS worst_lag_ms,
       bool_or(h.how IN ('answer', 'other')) AS pulled, any_value(w.gap) AS gap,
       any_value(coalesce(ph.phase, 'tail')) AS phase
FROM kept q JOIN chunks c ON c.k = q.k AND c.stream = q.stream AND c.sec >= q.s0 AND c.sec < q.s1
LEFT JOIN firsts h ON h.k = q.k AND h.node = q.node AND h.seq = c.seq
LEFT JOIN swarm w ON w.k = q.k AND w.stream = q.stream AND w.sec = c.sec
LEFT JOIN fs_phase ph ON ph.k = q.k AND ph.node = q.node AND ph.sec = c.sec * {grid_ms} // 1000
GROUP BY ALL ORDER BY q.arm, q.cell, q.seed, q.k, q.node, q.stream, c.sec;
"""


def sequences(conn, sample=SAMPLE, grid_ms=GRID_MS):
    """The sequences of `sample` streams a run (observations_sql) of
    forecast.prepare's selection on `conn`, a step `grid_ms` long (t0
    and the decoded paths count steps)."""
    rows = conn.execute(observations_sql(sample, grid_ms)).fetchall()
    out, cur, key, last = [], None, None, None
    for arm, cell, seed, k, peer, stream, sec, lag, pulled, gap, phase in rows:
        if (k, peer, stream) != key:
            key = (k, peer, stream)
            cur = Sequence(k, arm, cell, seed, peer, stream, sec, [], [], [])
            out.append(cur)
            last = None
        while cur.t0 + len(cur.obs) < sec:
            cur.obs.append(None)
            cur.swarm.append(0)
            cur.phase.append(None)
            last = None
        c = lag_class(None if lag >= 1e9 else lag)
        trend = 1 if last is None or c == last else (2 if c > last else 0)
        cur.obs.append((c, int(bool(pulled)), trend))
        cur.swarm.append(int(bool(gap)))
        cur.phase.append(phase)
        last = c
    return out


@dataclasses.dataclass
class Model:
    """An HSMM's parameters (module docs), in probabilities: `pi` the
    first state's, `a` transitions, `dur` each state's durations 1..D,
    `emit[j][z][c]` state j's emission of component c given swarm state z,
    and the fit's log likelihood by iteration."""
    pi: list
    a: list
    dur: list
    emit: list
    ll: list = dataclasses.field(default_factory=list)
    names: list = None


def lse(xs):
    m = max(xs)
    if m <= NEG:
        return NEG
    return m + math.log(sum(math.exp(x - m) for x in xs))


def logp(p):
    return math.log(p) if p > 0 else NEG


def init_model(k, d, rng):
    """A starting point: states ordered by lag, so the fit's states start
    apart (state j emits lag class j most)."""
    emit = []
    for j in range(k):
        per_z = []
        for z in range(2):
            comps = []
            for c, n in COMPONENTS:
                if c == "lag":
                    w = [1.0 + 4.0 * (i == min(j, n - 1)) + rng.random() for i in range(n)]
                else:
                    w = [1.0 + rng.random() for _ in range(n)]
                s = sum(w)
                comps.append([x / s for x in w])
            per_z.append(comps)
        emit.append(per_z)
    dur = [[1.0 / d] * d for _ in range(k)]
    a = [[1.0 / k] * k for _ in range(k)]
    return Model([1.0 / k] * k, a, dur, emit)


def emissions(model, seq):
    """Each second's log emission by state (0 without an observation)."""
    k = len(model.pi)
    le = [[[logp(p) for p in comp] for comp in model.emit[j][z]] for j in range(k) for z in range(2)]
    out = []
    for o, z in zip(seq.obs, seq.swarm):
        if o is None:
            out.append([0.0] * k)
            continue
        out.append([sum(le[j * 2 + z][c][o[c]] for c in range(len(o))) for j in range(k)])
    return out


def loglik(model, seq):
    """`seq`'s log likelihood under `model` (forward_backward's forward
    pass alone)."""
    return forward_backward(model, seq, forward_only=True)


def forward_backward(model, seq, forward_only=False):
    """The segment posteriors of `seq`: (log likelihood, occupancy
    gamma[t][j], first-state weights, transition counts, duration counts
    of complete segments); the log likelihood alone with
    `forward_only`."""
    k, d = len(model.pi), len(model.dur[0])
    T = len(seq.obs)
    e = emissions(model, seq)
    cum = [[0.0] * k]
    for t in range(T):
        cum.append([cum[-1][j] + e[t][j] for j in range(k)])
    la = [[logp(x) for x in row] for row in model.a]
    lpi = [logp(x) for x in model.pi]
    ld = [[logp(x) for x in row] for row in model.dur]
    # The duration's survival, for the censored last segment.
    ls = []
    for row in model.dur:
        s, acc = [0.0] * d, 0.0
        for i in range(d - 1, -1, -1):
            acc += row[i]
            s[i] = logp(acc)
        ls.append(s)
    F = [[NEG] * k for _ in range(T)]
    start = [[NEG] * k for _ in range(T)]
    for t in range(T):
        if t == 0:
            start[0] = lpi[:]
        else:
            start[t] = [lse([F[t - 1][i] + la[i][j] for i in range(k)]) for j in range(k)]
        for j in range(k):
            terms = []
            for dd in range(1, min(d, t + 1) + 1):
                s0 = t - dd + 1
                dur = ls[j][dd - 1] if t == T - 1 else ld[j][dd - 1]
                terms.append(start[s0][j] + dur + cum[t + 1][j] - cum[s0][j])
            F[t][j] = lse(terms)
    ll = lse(F[T - 1])
    if forward_only:
        return ll
    # G[s][j]: the rest given a segment of j starts at s; B[t][i]: given
    # one of i ends at t.
    G = [[NEG] * k for _ in range(T + 1)]
    B = [[NEG] * k for _ in range(T)]
    B[T - 1] = [0.0] * k
    for s in range(T - 1, -1, -1):
        for j in range(k):
            terms = []
            for dd in range(1, min(d, T - s) + 1):
                end = s + dd - 1
                dur = ls[j][dd - 1] if end == T - 1 else ld[j][dd - 1]
                terms.append(dur + cum[s + dd][j] - cum[s][j] + B[end][j])
            G[s][j] = lse(terms)
        if s >= 1:
            B[s - 1] = [lse([la[i][j] + G[s][j] for j in range(k)]) for i in range(k)]
    gamma_diff = [[0.0] * k for _ in range(T + 1)]
    durs = [[0.0] * d for _ in range(k)]
    for s in range(T):
        for j in range(k):
            if start[s][j] <= NEG:
                continue
            for dd in range(1, min(d, T - s) + 1):
                end = s + dd - 1
                dur = ls[j][dd - 1] if end == T - 1 else ld[j][dd - 1]
                lw = start[s][j] + dur + cum[s + dd][j] - cum[s][j] + B[end][j] - ll
                if lw < -30:
                    continue
                w = math.exp(lw)
                gamma_diff[s][j] += w
                gamma_diff[s + dd][j] -= w
                if end < T - 1:
                    durs[j][dd - 1] += w
    gamma, acc = [], [0.0] * k
    for t in range(T):
        acc = [acc[j] + gamma_diff[t][j] for j in range(k)]
        gamma.append(acc[:])
    first = [math.exp(lpi[j] + G[0][j] - ll) for j in range(k)]
    trans = [[0.0] * k for _ in range(k)]
    for t in range(T - 1):
        for i in range(k):
            if F[t][i] <= NEG:
                continue
            for j in range(k):
                lw = F[t][i] + la[i][j] + G[t + 1][j] - ll
                if lw > -30:
                    trans[i][j] += math.exp(lw)
    return ll, gamma, first, trans, durs


def normalized(w):
    s = sum(w)
    return [x / s for x in w] if s > 0 else [1.0 / len(w)] * len(w)


def fit(seqs, k=K, iters=ITERS, d=D, seed=0, say=None):
    """An HSMM fit to `seqs` by EM (module docs), from init_model's start
    (its jitter from Random(seed)); `say` takes each iteration's log
    likelihood."""
    model = init_model(k, d, random.Random(seed))
    for it in range(iters):
        pi = [PRIOR / k] * k
        a = [[PRIOR / k] * k for _ in range(k)]
        dur = [[PRIOR / d] * d for _ in range(k)]
        emit = [[[[PRIOR / n] * n for _, n in COMPONENTS] for _ in range(2)] for _ in range(k)]
        total = 0.0
        for seq in seqs:
            if not any(o is not None for o in seq.obs):
                continue
            ll, gamma, first, trans, durs = forward_backward(model, seq)
            total += ll
            for j in range(k):
                pi[j] += first[j]
                for i in range(k):
                    a[j][i] += trans[j][i]
                for dd in range(d):
                    dur[j][dd] += durs[j][dd]
            for t, (o, z) in enumerate(zip(seq.obs, seq.swarm)):
                if o is None:
                    continue
                for j in range(k):
                    g = gamma[t][j]
                    if g <= 0:
                        continue
                    for c in range(len(o)):
                        emit[j][z][c][o[c]] += g
        model = Model(normalized(pi), [normalized(r) for r in a], [normalized(r) for r in dur],
                      [[[normalized(c) for c in per_z] for per_z in per_j] for per_j in emit], model.ll + [total])
        if say:
            say(f"iteration {it}: log likelihood {total:.0f}")
    model.names = names(model)
    return model


def decode(model, seq):
    """The posterior's most likely state each second (occupancy's argmax:
    the forward-backward's marginal mode)."""
    _, gamma, _, _, _ = forward_backward(model, seq)
    return [max(range(len(g)), key=lambda j: g[j]) for g in gamma]


def emission_profile(model, j, z=0):
    """State j's mean lag class, pull share, rising-minus-falling trend
    share and missing share (the worst lag class) in swarm state `z` (by
    default open: a gap delays every peer alike)."""
    e = model.emit[j][z]
    return {"lag": sum(i * p for i, p in enumerate(e[0])), "pulled": e[1][1], "trend": e[2][2] - e[2][0],
            "missing": e[0][-1]}


def names(model):
    """The states' names, by their emissions out of a swarm gap
    (emission_profile): the less pulled half (by pull share) is on the
    push path, the rest pulled; within each, by mean lag class, the
    on-path ones are on_path_fresh then on_path_late, the pulled ones
    pulled_late then pulled_fresh from the latest (so a model of two
    states has on_path_fresh and pulled_late). More than two of a kind
    take a number."""
    k = len(model.pi)
    prof = [emission_profile(model, j) for j in range(k)]
    by_pull = sorted(range(k), key=lambda j: (prof[j]["pulled"], prof[j]["lag"]))
    on, pulled = by_pull[:(k + 1) // 2], by_pull[(k + 1) // 2:]
    out = [None] * k
    for group, labels, latest_first in ((on, ("on_path_fresh", "on_path_late"), False),
                                        (pulled, ("pulled_late", "pulled_fresh"), True)):
        for i, j in enumerate(sorted(group, key=lambda j: prof[j]["lag"], reverse=latest_first)):
            out[j] = labels[i] if i < len(labels) else f"{labels[-1]}_{i}"
    return out


def distance(a, i, b, j):
    """The total variation distance between state i of model `a` and
    state j of `b`, summed over the components, out of a gap."""
    return sum(sum(abs(p - q) for p, q in zip(x, y)) / 2 for x, y in zip(a.emit[i][0], b.emit[j][0]))


def matched(model, other):
    """`other`'s states matched to `model`'s: the permutation that brings
    their emissions closest (the least sum of `distance`s), as {other's
    state: model's}. A fit's state numbers are arbitrary, and names alone
    mislead where two fits split the same data differently."""
    from itertools import permutations

    k = len(model.pi)
    best = min(permutations(range(k)), key=lambda p: sum(distance(model, i, other, p[i]) for i in range(k)))
    return {best[i]: i for i in range(k)}


def runs_of(path):
    """A decoded path's dwell runs: (state, start, length, censored), a run
    censored that touches either end of the sequence."""
    out, s = [], 0
    for t in range(1, len(path) + 1):
        if t == len(path) or path[t] != path[s]:
            out.append((path[s], s, t - s, s == 0 or t == len(path)))
            s = t
    return out


# --- the description --------------------------------------------------------------------

# The states' order in the tables (`names`' for four states): the swarm
# table counts every one but the first as bad.
STATE_ORDER = ("on_path_fresh", "on_path_late", "pulled_fresh", "pulled_late")


def order(name):
    """A state's place in the tables: STATE_ORDER's, a numbered one last."""
    return STATE_ORDER.index(name) if name in STATE_ORDER else len(STATE_ORDER)
# Event alignment of the decoded states: seconds before and after, and the
# run-level kinds (each run's first after its start).
EVENT_BEFORE_S = 5
EVENT_AFTER_S = 20
EVENT_KINDS = ("uplink_rate", "depart")


def quantile(xs, p):
    """The `p` quantile of `xs` (linear between order statistics), None
    for none."""
    if not xs:
        return None
    xs = sorted(xs)
    i = p * (len(xs) - 1)
    lo = int(math.floor(i))
    hi = min(lo + 1, len(xs) - 1)
    return xs[lo] + (xs[hi] - xs[lo]) * (i - lo)


def rest(phase):
    return phase not in (None, "startup")


def states_table(model):
    """Each state's emissions (over both swarm states, and its mean lag
    class in a gap and out of one), its fitted duration's mean and median
    (s), the first state's probability and its transitions."""
    out = []
    for j, name in enumerate(model.names):
        lag = [sum(model.emit[j][z][0][i] for z in range(2)) / 2 for i in range(len(LAG_EDGES_MS) + 1)]
        prof = emission_profile(model, j)
        cdf, med = 0.0, None
        for d, p in enumerate(model.dur[j]):
            cdf += p
            if med is None and cdf >= 0.5:
                med = d + 1
        row = {"state": name, **{f"lag_{i}": lag[i] for i in range(len(lag))},
               "pulled": prof["pulled"], "rising": sum(model.emit[j][z][2][2] for z in range(2)) / 2,
               "falling": sum(model.emit[j][z][2][0] for z in range(2)) / 2,
               "lag_open": sum(i * p for i, p in enumerate(model.emit[j][0][0])),
               "lag_gap": sum(i * p for i, p in enumerate(model.emit[j][1][0])),
               "dur_mean_s": sum((d + 1) * p for d, p in enumerate(model.dur[j])), "dur_p50_s": med,
               "first": model.pi[j]}
        for i, other in enumerate(model.names):
            row[f"to_{other}"] = model.a[j][i]
        out.append(row)
    return sorted(out, key=lambda r: order(r["state"]))


def occupancy_table(model, seqs, paths):
    """Per (arm, cell, phase: startup or rest): the decoded seconds' share
    in each state and the state changes per minute."""
    acc = {}
    for seq, path in zip(seqs, paths):
        for t, j in enumerate(path):
            if seq.obs[t] is None:
                continue
            key = (seq.arm, seq.cell, "rest" if rest(seq.phase[t]) else "startup")
            a = acc.setdefault(key, {"n": 0, "changes": 0, **{n: 0 for n in model.names}})
            a["n"] += 1
            a[model.names[j]] += 1
            if t > 0 and path[t - 1] != j:
                a["changes"] += 1
    out = []
    for (arm, cell, phase), a in sorted(acc.items()):
        out.append({"arm": arm, "cell": cell, "phase": phase, "seconds": a["n"],
                    **{n: a[n] / a["n"] for n in sorted(model.names, key=order)},
                    "changes_per_min": 60 * a["changes"] / a["n"]})
    return out


def dwell_table(model, seqs, paths):
    """Per (arm, cell, state): the decoded dwell runs that start past the
    peer's startup: their count, the share censored (touching the
    sequence's end), and the uncensored runs' median, p90 and mean (s)."""
    acc = {}
    for seq, path in zip(seqs, paths):
        for j, s, n, cens in runs_of(path):
            if not rest(seq.phase[s]) and s > 0:
                continue
            if s == 0 and not rest(seq.phase[0]):
                continue
            a = acc.setdefault((seq.arm, seq.cell, model.names[j]), {"runs": 0, "censored": 0, "lens": []})
            a["runs"] += 1
            if cens:
                a["censored"] += 1
            else:
                a["lens"].append(n)
    out = []
    for (arm, cell, name), a in sorted(acc.items(), key=lambda x: (x[0][0], x[0][1], order(x[0][2]))):
        lens = a["lens"]
        out.append({"arm": arm, "cell": cell, "state": name, "runs": a["runs"],
                    "censored": a["censored"] / a["runs"], "p50_s": quantile(lens, 0.5),
                    "p90_s": quantile(lens, 0.9), "mean_s": sum(lens) / len(lens) if lens else None})
    return out


def transitions_table(model, seqs, paths):
    """Per (arm, cell, from state): where the decoded runs past the peer's
    startup go next, as shares, and how many left."""
    acc = {}
    for seq, path in zip(seqs, paths):
        runs = runs_of(path)
        for (j, s, _, _), (i, _, _, _) in zip(runs, runs[1:]):
            if not rest(seq.phase[s]):
                continue
            a = acc.setdefault((seq.arm, seq.cell, model.names[j]), {n: 0 for n in model.names})
            a[model.names[i]] += 1
    out = []
    for (arm, cell, name), a in sorted(acc.items(), key=lambda x: (x[0][0], x[0][1], order(x[0][2]))):
        n = sum(a.values())
        out.append({"arm": arm, "cell": cell, "from": name, "n": n,
                    **{f"to_{x}": a[x] / n for x in sorted(model.names, key=order)}})
    return out


def events_table(model, seqs, paths, events):
    """Per (arm, cell, kind, offset s): the decoded states' shares around
    each run's first event of each kind (`events`: {k: {kind: t_ms}}), the
    peer's own parent changes of the stream (`events`' ("parent", k,
    peer, stream): [t_ms]) and its join ((k, peer): arrive_ms)."""
    acc = {}

    def add(seq, kind, at_s, path):
        for t, j in enumerate(path):
            off = seq.t0 + t - at_s
            if seq.obs[t] is None or off < -EVENT_BEFORE_S or off >= EVENT_AFTER_S:
                continue
            a = acc.setdefault((seq.arm, seq.cell, kind, off), {"n": 0, **{n: 0 for n in model.names}})
            a["n"] += 1
            a[model.names[j]] += 1

    for seq, path in zip(seqs, paths):
        for kind, t_ms in events.get(seq.k, {}).items():
            add(seq, kind, t_ms // GRID_MS, path)
        for t_ms in events.get(("parent", seq.k, seq.peer, seq.stream), []):
            add(seq, "parent_change", t_ms // GRID_MS, path)
        arrive = events.get((seq.k, seq.peer))
        if arrive is not None and arrive > 0:
            add(seq, "peer_join", arrive // GRID_MS, path)
    out = []
    for (arm, cell, kind, off), a in sorted(acc.items()):
        out.append({"arm": arm, "cell": cell, "kind": kind, "off_s": off, "n": a["n"],
                    **{n: a[n] / a["n"] for n in sorted(model.names, key=order)}})
    return out


def swarm_table(model, seqs, paths):
    """Per (arm, cell, swarm state): the seconds' share in a gap, the
    share of peer-seconds in a bad state, and the overdispersion of a
    (run, stream, second)'s count of peers in a bad state: its variance
    over those seconds against the binomial variance of independent peers
    at each second's group size and the swarm state's rate (1 when peers
    are independent once the gap is known; past 1, peers share more)."""
    groups = {}
    for seq, path in zip(seqs, paths):
        for t, j in enumerate(path):
            if seq.obs[t] is None or not rest(seq.phase[t]):
                continue
            key = (seq.arm, seq.cell, seq.k, seq.stream, seq.t0 + t)
            g = groups.setdefault(key, [seq.swarm[t], 0, 0])
            g[1] += 1
            g[2] += model.names[j] != STATE_ORDER[0]
    acc = {}
    for (arm, cell, _, _, _), (z, n, bad) in groups.items():
        acc.setdefault((arm, cell, z), []).append((n, bad))
    out = []
    totals = {}
    for (arm, cell, z), gs in acc.items():
        totals[(arm, cell)] = totals.get((arm, cell), 0) + len(gs)
    for (arm, cell, z), gs in sorted(acc.items()):
        n_all = sum(n for n, _ in gs)
        rate = sum(b for _, b in gs) / n_all
        multi = [(n, b) for n, b in gs if n >= 2]
        if multi:
            mean = sum(b - n * rate for n, b in multi) / len(multi)
            var = sum((b - n * rate - mean) ** 2 for n, b in multi) / len(multi)
            binom = sum(n * rate * (1 - rate) for n, _ in multi) / len(multi)
            disp = var / binom if binom > 0 else None
        else:
            disp = None
        out.append({"arm": arm, "cell": cell, "swarm": "gap" if z else "open", "seconds": len(gs),
                    "second_share": len(gs) / totals[(arm, cell)], "peer_seconds": n_all, "bad_share": rate,
                    "peers_per_second": n_all / len(gs), "overdispersion": disp})
    return out


def stability_table(model, halves, seqs, paths):
    """The fit's stability across seeds: each half fit's (odd and even
    seeds) states matched to the pooled fit's (`matched`), under the
    pooled state they match, with their own name, their `distance` from
    it, mean lag class, pull share, missing share and trend out of a gap,
    fitted mean duration and the probability of following themselves;
    then, under the pooled fit, the spread over seeds of each state's
    share of the rest phase's seconds (min, median and max over the seeds
    of each arm and cell)."""
    out = []
    for label, m in [("pooled", model), *(("odd seeds", halves[1]), ("even seeds", halves[0]))]:
        if m is None:
            continue
        match = matched(model, m)
        for j, name in enumerate(m.names):
            prof = emission_profile(m, j)
            out.append({"state": model.names[match[j]], "fit": label, "own_name": name,
                        "distance": distance(model, match[j], m, j), "lag": prof["lag"], "pulled": prof["pulled"],
                        "missing": prof["missing"], "trend": prof["trend"],
                        "dur_mean_s": sum((d + 1) * p for d, p in enumerate(m.dur[j])), "self": m.a[j][j]})
    out.sort(key=lambda r: (order(r["state"]),
                            r["fit"]))
    share = {}
    for seq, path in zip(seqs, paths):
        for t, j in enumerate(path):
            if seq.obs[t] is None or not rest(seq.phase[t]):
                continue
            a = share.setdefault((seq.arm, seq.cell, seq.seed), {"n": 0, **{n: 0 for n in model.names}})
            a["n"] += 1
            a[model.names[j]] += 1
    spread = {}
    for (arm, cell, _), a in share.items():
        for n in model.names:
            spread.setdefault((arm, cell, n), []).append(a[n] / a["n"])
    seeds = []
    for (arm, cell, n), xs in sorted(spread.items(), key=lambda x: (x[0][0], x[0][1], order(x[0][2]))):
        seeds.append({"arm": arm, "cell": cell, "state": n, "seeds": len(xs), "min": min(xs),
                      "p50": quantile(xs, 0.5), "max": max(xs)})
    return out, seeds


def event_times(conn):
    """The events the decoded states align on, for forecast.prepare's
    selection: {k: {kind: first t_ms}}, {("parent", k, node, stream):
    [t_ms]} and {(k, node): arrive_ms}."""
    kinds = ", ".join(f"'{k}'" for k in EVENT_KINDS)
    out = {}
    for k, kind, t in conn.execute(f"""
SELECT e.k, e.kind, min(e.t_ms)::BIGINT FROM events e JOIN fs_jobs j USING (k, src)
WHERE e.kind IN ({kinds}) AND e.t_ms > 0 GROUP BY ALL""").fetchall():
        out.setdefault(k, {})[kind] = t
    for k, node, stream, t in conn.execute("""
SELECT p.k, p.node, p.stream, p.t_ms FROM parents p JOIN fs_jobs j USING (k, src) WHERE p.t_ms > 0
ORDER BY ALL""").fetchall():
        out.setdefault(("parent", k, node, stream), []).append(t)
    for k, node, arrive in conn.execute("""
SELECT v.k, v.node, v.arrive_ms FROM viewers v JOIN fs_jobs j USING (k, src)""").fetchall():
        out[(k, node)] = arrive
    return out


def load_states(conn, model, seqs, paths):
    """The decoded states as fs_states (k, peer, stream, sec, state) on
    `conn`, through a CSV file (the module inserts a row at a time)."""
    import csv
    import tempfile

    with tempfile.TemporaryDirectory(prefix="regimes-") as d:
        f = Path(d) / "states.csv"
        with open(f, "w", newline="") as out:
            w = csv.writer(out)
            for seq, path in zip(seqs, paths):
                for t, j in enumerate(path):
                    w.writerow((seq.k, seq.peer, seq.stream, seq.t0 + t, model.names[j]))
        conn.execute("CREATE OR REPLACE TEMP TABLE fs_states AS SELECT * FROM read_csv(?, header = false, columns = "
                     "{'k': 'VARCHAR', 'peer': 'BIGINT', 'stream': 'BIGINT', 'sec': 'BIGINT', 'state': 'VARCHAR'})",
                     [str(f)])


def regimes(sel, conn=None, sample=SAMPLE, decode_sample=DECODE_SAMPLE, iters=ITERS, say=None):
    """The regime description of `sel`'s jobs (forecast.Selection): the
    pooled fit on `sample` streams a run (every FIT_THIN-th sequence of
    them), its halves (odd and even seeds),
    and the decoded description of `decode_sample` streams a run, with the
    decoded states' share of the decisions' failures (`availability`,
    forecast's explained scoring with the asked peer's decoded state as a
    class) and how much receivers agree beside peers (`by_receiver`).
    Returns {table: [dict]} and the pooled model."""
    from . import forecast

    say = say or (lambda m: None)
    with forecast.connected(conn) as c:
        forecast.prepare(c, sel)
        fit_seqs = sequences(c, sample)[::FIT_THIN]
        say(f"fit: {len(fit_seqs)} sequences, {sum(len(s.obs) for s in fit_seqs)} seconds")
        model = fit(fit_seqs, iters=iters, say=say)
        halves = [fit([s for s in fit_seqs if s.seed % 2 == r], iters=iters, say=say) if
                  any(s.seed % 2 == r for s in fit_seqs) else None for r in (0, 1)]
        seqs = sequences(c, decode_sample)
        say(f"decode: {len(seqs)} sequences")
        paths = [decode(model, s) for s in seqs]
        events = event_times(c)
        load_states(c, model, seqs, paths)
        out = {"states": states_table(model), "occupancy": occupancy_table(model, seqs, paths),
               "dwell": dwell_table(model, seqs, paths), "transitions": transitions_table(model, seqs, paths),
               "events": events_table(model, seqs, paths, events), "swarm": swarm_table(model, seqs, paths)}
        out["stability"], out["seed_spread"] = stability_table(model, halves, seqs, paths)
        out["availability"] = c.execute(forecast.state_explained_sql()).fetchall()
        out["availability"] = [dict(zip([d[0] for d in c.description], r)) for r in out["availability"]]
        out["by_receiver"] = [dict(zip([d[0] for d in c.description], r))
                              for r in c.execute(forecast.agreement_sql()).fetchall()]
    return out, model


# --- the learning curve -------------------------------------------------------------

# The seeds the learning curve holds out, and the distance (`distance`)
# within which a fit's state still stands for the reference's: the
# pooled fit's halves land 0.04-0.26 from it where they find the same
# state and 0.58-1.52 where they don't.
HELD_OUT_SEEDS = (7, 8)
SURVIVES = 0.4


def subsample(seqs, n):
    """The first `n` of `seqs` by a hash of their (run, peer, stream):
    the same each time, and nested (a smaller sample is in every larger
    one)."""
    import hashlib

    return sorted(seqs, key=lambda s: hashlib.sha1(f"{s.k}/{s.peer}/{s.stream}".encode()).hexdigest())[:n]


def per_observation(model, seqs):
    """The mean log likelihood of an observation of `seqs` under
    `model`."""
    n = sum(o is not None for s in seqs for o in s.obs)
    return sum(loglik(model, s) for s in seqs if any(o is not None for o in s.obs)) / n


def timed_fit(job):
    """A learning curve job: (label, its sequences, iterations) -> (label,
    the fit, its wall time in s)."""
    import time

    label, seqs, iters = job
    t0 = time.monotonic()
    m = fit(seqs, iters=iters)
    return label, m, time.monotonic() - t0


def learning_curve(seqs, sizes, iters=ITERS, workers=None, held_out=HELD_OUT_SEEDS, everything=False):
    """How the fit grows with its data: `seqs`' sequences of the
    `held_out` seeds kept apart, and for each of `sizes` the first that
    many of the rest (`subsample`) fit pooled and by halves (even and odd
    seeds), the fits run in `workers` processes. A row a size: its
    sequences and observations, the pooled fit's wall time, the held-out
    sequences' log likelihood per observation under it, and for the
    reference fit's pulled_fresh (the largest size's pooled fit's) the
    `distance` of the matching state of the pooled fit and of each half,
    and whether both halves have it (within SURVIVES). With
    `everything`, a last row for a pooled fit on every sequence, the
    held-out ones too (its held-out likelihood in-sample)."""
    import concurrent.futures as cf

    test = [s for s in seqs if s.seed in held_out]
    pool = [s for s in seqs if s.seed not in held_out]
    jobs, subs = [], {}
    for n in sizes:
        subs[n] = subsample(pool, n)
        jobs += [((n, "pooled"), subs[n], iters),
                 ((n, "even"), [s for s in subs[n] if s.seed % 2 == 0], iters),
                 ((n, "odd"), [s for s in subs[n] if s.seed % 2 == 1], iters)]
    if everything:
        subs["all"] = seqs
        jobs.append((("all", "pooled"), seqs, iters))
    with cf.ProcessPoolExecutor(workers) as ex:
        fits = {label: (m, wall) for label, m, wall in ex.map(timed_fit, jobs)}
    ref = fits[(max(sizes), "pooled")][0]
    ref_j = ref.names.index("pulled_fresh")

    def gap(m):
        mine = next(j for j, i in matched(ref, m).items() if i == ref_j)
        return distance(ref, ref_j, m, mine)

    rows = []
    for n in [*sizes, *(["all"] if everything else [])]:
        m, wall = fits[(n, "pooled")]
        row = {"sequences": len(subs[n]), "observations": sum(o is not None for s in subs[n] for o in s.obs),
               "fit_s": wall, "held_out_ll": per_observation(m, test), "in_sample": n == "all",
               "pulled_fresh": gap(m)}
        if n != "all":
            row["even"], row["odd"] = gap(fits[(n, "even")][0]), gap(fits[(n, "odd")][0])
            row["both_halves"] = row["even"] <= SURVIVES and row["odd"] <= SURVIVES
        rows.append(row)
    return rows


def regimes_text(tables):
    """The description's text: each table."""
    from . import engine, forecast

    out = []
    for name, rows in tables.items():
        out.append(f"## regimes: {name}\n")
        if rows:
            names = list(rows[0])
            out.append(engine.render_table(names, [[forecast.fmt(r.get(n)) for n in names] for r in rows]))
        out.append("\n")
    return "".join(out)
