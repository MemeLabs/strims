#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's selections over a scratch cache, suite indexes and farm state,
as two farm runs of several suite tiers leave them (an A/B and its
confirmation, each a browsers and a net tier, their cells sharing names,
the A/B reusing rows of the confirmation's); its queries and joins over
a scratch cache of both sims' tables (Queries); and its pulls of a farm
run's samples from a scratch S3, the aws CLI stubbed (SamplesPull):

  python3 hack/sim-db/simdb_test.py

Needs the pinned DuckDB CLI (simdb.py duckdb fetches it)."""

import argparse
import importlib.util
import io
import json
import math
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

spec = importlib.util.spec_from_file_location("simdb", Path(__file__).with_name("simdb.py"))
simdb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(simdb)

A, B = "farm/sa0001-00+3more-aaaa", "farm/sb0001-00+3more-bbbb"
GONE = "farm/sc0001-00/i-1/0"
TIER = {"cells": [{"name": "star-3-5", "sim": "strims-sim"}, {"name": "halve-7-10", "sim": "strims-sim"},
                  {"name": "star-7-5", "sim": "strims-sim"}]}
INDEXES = {
    "sa0001": TIER,
    "sa0002": {"cells": [{"name": "org-star-7-5", "sim": "ppspp"}],
               "knob": {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"}, "values": ["off", "a"]},
    "sb0001": TIER,
}
# Kept apart by suite.py until its run started.
PENDING = {"sb0002": {"cells": [{"name": "org-star-7-5", "sim": "ppspp"}]}}
MATRICES = {A: ["sa0001-00", "sa0001-01", "sa0001-02", "sa0002-00"],
            B: ["sb0001-00", "sb0001-01", "sb0001-02", "sb0002-00"]}
# source: (k, matrix, arm, env)
STORED = {
    f"{A}/i-1/0": [("k1", "sa0001-00", "main", {}), ("k2", "sa0001-00", "fix", {}),
                   ("k3", "sa0001-01", "main", {}), ("k4", "sa0002-00", "x", {"SIM_EXP": "off"}),
                   ("k5", "sa0002-00", "x", {"SIM_EXP": "a"}),
                   # the verify copy of a job the farm reused from B's run
                   ("kr", "sa0001-01", "main", {})],
    # a second instance's copy of k2
    f"{A}/i-2/0": [("k2", "sa0001-00", "fix", {})],
    f"{B}/i-1/0": [("k6", "sb0001-00", "main", {}), ("k7", "sb0002-00", "x", {}),
                   ("kr", "sb0001-01", "main", {}), ("kq", "sb0001-01", "main", {}),
                   ("kz", "sb0001-02", "main", {})],
    "suite/wt/inv": [("k8", "star-3-5", "strims-sim-0123", {})],
}
# matrix: the rows its run reused, (k, src, arm, from); GONE's flush has
# left S3.
REUSED = {
    "sa0001-01": [("kr", f"{B}/i-1/0", "main", "sb0001-01"), ("kq", f"{B}/i-1/0", "main", "sb0001-01"),
                  ("kg", GONE, "main", "sc0001-00")],
    # the matrix runs nothing itself, and names the arm its own way
    "sa0001-02": [("kz", f"{B}/i-1/0", "base", "sb0001-02")],
}
# source: (k, sec)
SECONDS = {f"{B}/i-1/0": [("kq", 0), ("kq", 1), ("kz", 0)]}


class Selection(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        self.pulled = []
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(simdb, "pull_sources", lambda s, reads=False: self.pulled.extend(s) or list(s)),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
            p.start()
            self.addCleanup(p.stop)
        runs = root / "target" / "sim-suite" / "farm-runs"
        (runs / "pending").mkdir(parents=True)
        for d, idx in ((runs, INDEXES), (runs / "pending", PENDING)):
            for run, x in idx.items():
                (d / f"{run}.json").write_text(json.dumps(x))
        farm = root / "target" / "sim-farm"
        for src, ms in MATRICES.items():
            (farm / src.split("/")[1]).mkdir(parents=True)
            (farm / src.split("/")[1] / "matrices").write_text("".join(f"{m}\n" for m in ms))
        lines = {}
        for src, rows in STORED.items():
            for k, m, arm, env in rows:
                lines.setdefault(m, []).append({"key": k, "arm": arm, "env": env, "src": src, "row": {"passed": 1}})
        for m, rows in REUSED.items():
            lines.setdefault(m, []).extend({"key": k, "arm": arm, "src": s, "from": f, "row": {}}
                                           for k, s, arm, f in rows)
        for m, recs in lines.items():
            (farm / m).mkdir(parents=True, exist_ok=True)
            # a torn last line, as a writer that died leaves it
            (farm / m / "index.jsonl").write_text("".join(json.dumps(r) + "\n" for r in recs) + '{"key": "k')
        q = simdb.q
        for src, rows in STORED.items():
            d = simdb.cache_root() / "cache" / src
            d.mkdir(parents=True)
            vals = ", ".join(f"({q(k)}, {q(src)}, {q(m)}, {q(arm)}, 1, {q(json.dumps(env))})"
                             for k, m, arm, env in rows)
            simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, cell, arm, seed, env)) "
                       f"TO {q(d / 'runs.parquet')} (FORMAT parquet);", out=None)
            if src in SECONDS:
                vals = ", ".join(f"({q(k)}, {q(src)}, {s})" for k, s in SECONDS[src])
                simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, sec)) "
                           f"TO {q(d / 'seconds.parquet')} (FORMAT parquet);", out=None)

    def select(self, src=None, cell=None, sql="SELECT src, k, cell, arm FROM runs"):
        v, _ = simdb.views(argparse.Namespace(src=src, cell=cell, arm=None, seed=None), derived=False)
        return simdb.duck(v + "\n" + sql + ";")

    def runs(self, src=None, cell=None):
        """{(src, k, cell): arm}"""
        return {(r["src"], r["k"], r["cell"]): r["arm"] for r in self.select(src, cell)}

    def cells(self, src=None, cell=None):
        """{cell: {(k, arm)}}"""
        out = {}
        for (_, k, c), arm in self.runs(src, cell).items():
            out.setdefault(c, set()).add((k, arm))
        return out

    def test_src_keeps_every_run_and_its_name(self):
        every = self.runs()
        farm = [s for s in STORED if s.startswith("farm/")]
        for src, srcs in (([f"{A}*"], [f"{A}/i-1/0", f"{A}/i-2/0"]), ([f"{B}*"], [f"{B}/i-1/0"]),
                          ([f"{A}/i-1/0"], [f"{A}/i-1/0"]), (["farm/*"], farm), (["suite/*"], ["suite/wt/inv"]),
                          ([f"{A}*", f"{B}*"], farm)):
            want = {(k, m) for s in srcs for k, m, _, _ in STORED[s]}
            want |= {(k, m) for s in srcs if s.startswith("farm/") for m in MATRICES[s.rsplit("/", 2)[0]]
                     for k, held, _, _ in REUSED.get(m, []) if held != GONE}
            got = self.select(src, sql="SELECT k, matrix FROM runs")
            self.assertEqual({(r["k"], r["matrix"]) for r in got}, want, src)
            named = {(k, c): arm for (_, k, c), arm in every.items()}
            for (_, k, c), arm in self.runs(src).items():
                self.assertEqual(arm, named.get((k, c)), (src, k, c))

    def test_cells_are_named_by_their_suite_run(self):
        self.assertEqual(set(self.cells()), {
            "star-3-5@sa0001", "halve-7-10@sa0001", "star-7-5@sa0001", "org-star-7-5@sa0002",
            "star-3-5@sb0001", "halve-7-10@sb0001", "star-7-5@sb0001", "org-star-7-5@sb0002", "star-3-5"})
        self.assertEqual(set(self.cells([f"{A}*"])),
                         {"star-3-5@sa0001", "halve-7-10@sa0001", "star-7-5@sa0001", "org-star-7-5@sa0002"})

    def test_cell_glob_with_src(self):
        got = self.runs([f"{A}*"], ["org-star-7-5@sa0002"])
        self.assertEqual(sorted(got.values()), ["a", "off"])
        self.assertEqual(self.runs([f"{A}*"], ["org-*"]), got)
        self.assertEqual(self.runs(None, ["*@sa0002"]), got)
        self.assertEqual(set(self.cells(None, ["star-3-5"])), {"star-3-5@sa0001", "star-3-5@sb0001", "star-3-5"})
        self.assertEqual(set(self.cells([f"{B}*"], ["star-3-5"])), {"star-3-5@sb0001"})
        picked = self.select(None, None, "SELECT DISTINCT cell FROM pick(['star-3-5', 'org-star-7-5@sb0002'], [])")
        self.assertEqual({r["cell"] for r in picked},
                         {"star-3-5@sa0001", "star-3-5@sb0001", "star-3-5", "org-star-7-5@sb0002"})

    def test_a_key_counts_once_a_cell(self):
        got = self.select(None, None, "SELECT cell, k, count(*) n FROM runs GROUP BY ALL")
        self.assertEqual({(r["cell"], r["k"]): r["n"] for r in got if r["k"] in ("k2", "kr")},
                         {("star-3-5@sa0001", "k2"): 1, ("halve-7-10@sa0001", "kr"): 1,
                          ("halve-7-10@sb0001", "kr"): 1})

    def test_a_run_holds_the_rows_it_reused(self):
        for src in (None, [f"{A}*"], [f"{A}/i-1/0"]):
            got = self.cells(src, ["*@sa0001"])
            self.assertEqual(got["halve-7-10@sa0001"], {("k3", "main"), ("kr", "main"), ("kq", "main")}, src)
            self.assertEqual(got["star-7-5@sa0001"], {("kz", "base")}, src)
            seconds = self.select(src, ["*@sa0001"],
                                  "SELECT r.cell, count(*) n FROM seconds JOIN runs r USING (k, src) GROUP BY ALL")
            self.assertEqual({r["cell"]: r["n"] for r in seconds}, {"halve-7-10@sa0001": 2, "star-7-5@sa0001": 1})
        self.assertEqual(set(self.pulled), {GONE})
        self.assertEqual(self.cells([f"{B}*"])["star-7-5@sb0001"], {("kz", "main")})
        self.assertNotIn("halve-7-10@sa0001", self.cells([f"{B}*"]))


# Queries' sources: a ppspp run's (its viewers name their node `node`), a
# strims-sim run's (`host`), and a second copy of a key the first holds in
# another cell, with fewer of its tables (as a farm run reuses a row).
P, S, Q = "suite/wt/p", "suite/wt/s", "suite/wt/q"
# source: (k, cell, arm, seed)
RUNS = {P: [("kp1", "c", "x", 1), ("kp2", "c", "x", 2), ("kd", "d1", "x", 3)],
        S: [("ks1", "c", "x", 4)],
        Q: [("kd", "d2", "x", 3)]}
# k: (viewer, node)
VIEWERS = {"kp1": [(0, 1), (1, 2)], "kp2": [(0, 1)], "kd": [(0, 1)], "ks1": [(0, 3), (1, 5)]}
# k: (t_ms, kind, node); node 0 is a seed, no viewer
EVENTS = {"kp1": [(-1000, "join", 1), (-1000, "join", 2), (4000, "join", 2),
                  (1000, "first_read", 1), (3000, "first_read", 2)],
          "kp2": [(2000, "first_read", 1)],
          "ks1": [(1500, "first_read", 3), (2500, "first_read", 5), (0, "latency", 0)]}
# k: (viewer, first read's t_ms, lag_ms): a read every 100 ms to 4900
READ_RUNS = {"kp1": [(0, 1000, 100), (1, 3000, 300)], "kp2": [(0, 2000, 200)], "kd": [(0, 0, 50)],
             "ks1": [(0, 1500, 150), (1, 2500, 250)]}
READS = {k: [(v, t, lag) for v, start, lag in rs for t in range(start, 5000, 100)] for k, rs in READ_RUNS.items()}
# (k, sec): phase; a second without one is the tail's
PHASES = {**{("kp1", s): "startup" for s in range(3)}, **{("kp1", s): "steady" for s in (3, 4)},
          **{("kp2", s): "startup" for s in range(5)}}
# k: the viewers present each second (analyze.events' present_join and
# present_leave): kp1's second viewer counts from 2 s
PRESENT = {"kp1": [1, 1, 2, 2, 2], "kp2": [1] * 5, "kd": [1] * 5, "ks1": [2] * 5}


def qc(xs, p):
    """quantile_cont"""
    xs = sorted(xs)
    at = p * (len(xs) - 1)
    i = math.floor(at)
    return xs[i] + (xs[min(i + 1, len(xs) - 1)] - xs[i]) * (at - i)


class Queries(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(simdb, "derive", lambda srcs: None),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
            p.start()
            self.addCleanup(p.stop)
        for src, rows in RUNS.items():
            self.put(src, "runs", ["cell", "arm", "seed", "env", "bin_ms"],
                     [(k, cell, arm, seed, "{}", 100) for k, cell, arm, seed in rows])
            keys = [r[0] for r in rows]
            self.put(src, "reads", ["viewer", "t_ms", "seq", "lag_ms"],
                     [(k, v, t, t // 100, lag) for k in keys for v, t, lag in READS[k]])
            self.derived(src, "derived_events", [
                [k, src, t, kind, value] for k in keys for t, kind, value in simdb.analyze().events(
                    {"present": PRESENT[k], "seed_up": [(0, 1000)]},
                    {"healthy": [True] * 5, "onset": None, "recovered": None})])
        self.put(P, "viewers", ["viewer", "node"], [(k, v, n) for k in ("kp1", "kp2", "kd") for v, n in VIEWERS[k]])
        self.put(S, "viewers", ["viewer", "host"], [("ks1", v, n) for v, n in VIEWERS["ks1"]])
        for src, keys in ((P, ("kp1", "kp2")), (S, ("ks1",))):
            self.put(src, "events", ["t_us", "kind", "node", "peer", "value", "detail"],
                     [(k, t * 1000, kind, n, -1, 0, "") for k in keys for t, kind, n in EVENTS[k]])
        self.put(P, "seconds", ["sec", "present"], [("kp1", s, 2) for s in range(5)])
        self.put(P, "viewer_seconds", ["viewer", "sec", "lag_ms", "frames"],
                 [("kp1", v, s, lag if s * 1000 >= start else -1, 10 if s * 1000 >= start else 0)
                  for v, start, lag in READ_RUNS["kp1"] for s in range(5)])
        # each viewer's node hits a chunk a bin from its first read; the seed none
        self.put(P, "node_bins", ["bin", "t_ms", "node", "viewer", "hits"],
                 [("kp1", t // 100, t, n, v, int(v >= 0 and t >= start))
                  for n, v, start in ((1, 0, 1000), (2, 1, 3000), (0, -1, 0)) for t in range(0, 5000, 100)])
        self.derived(P, "verdict_seconds", [[k, P, s, ph, True, False, False, False, False, 1.0, 1.0, 0.0]
                                            for (k, s), ph in PHASES.items()])

    def put(self, src, table, cols, rows):
        d = simdb.cache_root() / "cache" / src
        d.mkdir(parents=True, exist_ok=True)
        lit = lambda x: simdb.q(x) if isinstance(x, str) else repr(x)
        vals = ", ".join("(" + ", ".join(lit(x) for x in (r[0], src, *r[1:])) + ")" for r in rows)
        simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t({', '.join(['k', 'src', *cols])})) "
                   f"TO {simdb.q(d / f'{table}.parquet')} (FORMAT parquet);", out=None)

    def derived(self, src, table, rows):
        simdb.write_table(simdb.derived_dir(table) / (src.replace("/", "~") + ".parquet"), simdb.DERIVED[table], rows)

    def args(self, **kw):
        return argparse.Namespace(**{"src": None, "cell": None, "arm": None, "seed": None, "bin": 1000,
                                     "since": None, "until": None, "json": True, **kw})

    def run_cmd(self, cmd, **kw):
        out = io.StringIO()
        with mock.patch.object(sys, "stdout", out):
            cmd(self.args(**kw))
        return json.loads(out.getvalue())

    def sql(self, query, src=None):
        v, _ = simdb.views(self.args(src=src))
        return simdb.duck(v + "\n" + query + ";")

    def aligned(self, event, field, per, nth=1, before=1, after=2, node=None):
        got = self.run_cmd(simdb.cmd_aligned, event=event, field=field, per=per, nth=nth, before=before,
                           after=after, node=node)
        return {(r["cell"], r["rel_ms"]): (r["n"], r["p10"], r["p50"], r["p90"]) for r in got}

    def refused(self, event, field, per):
        with self.assertRaises(SystemExit) as cm:
            self.run_cmd(simdb.cmd_aligned, event=event, field=field, per=per, nth=1, before=1, after=2, node=None)
        return str(cm.exception.code)

    def test_pooled_columns_are_each_groups(self):
        cells = {}
        for rows in RUNS.values():
            for k, cell, arm, _ in rows:
                cells.setdefault((cell, arm), set()).add(k)
        group = {"none": lambda k, v, t: None, "bin": lambda k, v, t: t // 1000 * 1000,
                 "viewer": lambda k, v, t: v, "phase": lambda k, v, t: PHASES.get((k, t // 1000), "tail")}
        for by, g in group.items():
            want = {}
            for (cell, arm), keys in cells.items():
                for k in keys:
                    for v, t, lag in READS[k]:
                        want.setdefault((cell, arm, g(k, v, t)), {}).setdefault(k, []).append(lag)
            want = {key: (len(runs), sum(map(len, runs.values())),
                          sum(qc(x, 0.99) for x in runs.values()) / len(runs)) for key, runs in want.items()}
            got = {(r["cell"], r["arm"], r["g"]): (r["runs"], r["reads"], r["mean_run_p99"])
                   for r in self.run_cmd(simdb.cmd_pooled, by=by)}
            self.assertEqual(set(got), set(want), by)
            for key, (runs, reads, p99) in want.items():
                self.assertEqual(got[key][:2], (runs, reads), (by, key))
                self.assertAlmostEqual(got[key][2], p99, 6, (by, key))
            if by == "bin":
                # the 1000 ms bin of cell c has reads of kp1 and ks1 only
                self.assertEqual(got[("c", "x", 1000)][0], 2)

    def test_aligned_per_node_on_each_nodes_own_event(self):
        # every node of cell c reads its own lag from its own first read on
        self.assertEqual(self.aligned("first_read", "lag_p50", "node"),
                         {("c", 0): (5, 120.0, 200.0, 280.0), ("c", 1000): (5, 120.0, 200.0, 280.0)})
        # a run's first first read: kp1's second viewer reads after 2 s
        run = self.aligned("first_read", "lag_p50", "run")
        self.assertEqual({rel: n for (_, rel), (n, *_) in run.items()}, {0: 3, 1000: 3})
        self.assertEqual(run[("c", 0)][2], 150.0)
        # a node's own second join: kp1's node 2 joins again at 4 s
        self.assertEqual(self.aligned("join", "lag_p50", "node", nth=2),
                         {("c", -1000): (1, 300.0, 300.0, 300.0), ("c", 0): (1, 300.0, 300.0, 300.0)})
        self.assertEqual(self.aligned("join", "lag_p50", "node", nth=1, node=2, before=0, after=1), {})
        # node_bins: each viewer's node from its own first read; the seed has none
        self.assertEqual(self.aligned("first_read", "hits", "node", after=1),
                         {("c", -1000): (2, 0.0, 0.0, 0.0), ("c", 0): (2, 10.0, 10.0, 10.0)})
        # viewer_seconds by node: frames summed, lag_ms the bin's worst second with a frame
        self.assertEqual(self.aligned("first_read", "frames", "node"),
                         {("c", -1000): (2, 0.0, 0.0, 0.0), ("c", 0): (2, 10.0, 10.0, 10.0),
                          ("c", 1000): (2, 10.0, 10.0, 10.0)})
        self.assertEqual(self.aligned("first_read", "lag_ms", "node", after=1),
                         {("c", 0): (2, 120.0, 200.0, 280.0)})
        # the seed's node reads nothing
        self.assertEqual(self.aligned("latency", "lag_p50", "node"), {})

    def test_aligned_per_node_refuses_run_wide_fields_and_events(self):
        for field, what in (("healthy", "a verdict column"), ("present", "a seconds column")):
            msg = self.refused("first_read", field, "node")
            self.assertIn(f"{field} is run-wide ({what})", msg)
            self.assertIn("node_bins", msg)
        self.assertIn("start is run-wide", self.refused("start", "lag_p99", "node"))
        self.assertIn("no nope event", self.refused("nope", "lag_p99", "node"))
        for field, table in (("frames", "viewer_seconds"), ("hits", "node_bins")):
            self.assertIn(f"{field} is a node's (a {table} column): align it --per node",
                          self.refused("first_read", field, "run"))

    def test_derived_kinds_are_apart_from_the_sims(self):
        # every kind analyze.events makes: a viewer joins at 5 s and one
        # leaves at 15, the seed's uplink halves at 20 s and comes back at
        # 30, and 16-25 s are unhealthy, a collapse
        healthy = [True] * 16 + [False] * 10 + [True] * 14
        got = simdb.analyze().events({"present": [3] * 5 + [5] * 10 + [4] * 25,
                                      "seed_up": [(0, 1000), (20, 500), (30, 1000)]},
                                     {"healthy": healthy, "onset": 16, "recovered": 26})
        self.assertEqual({kind for _, kind, _ in got}, set(simdb.DERIVED_KINDS))
        self.assertIn((5, "present_join", 2), got)
        self.assertIn((15, "present_leave", 1), got)
        # a run's nth join is the sims' (kp1's node 2 again at 4 s), not the
        # viewers present rising (kp1 at 2 s)
        self.assertEqual(self.aligned("join", "lag_p50", "run", nth=3, before=0, after=1),
                         {("c", 0): (1, 200.0, 200.0, 200.0)})
        self.assertEqual(self.aligned("present_join", "lag_p50", "run", before=0, after=1),
                         {("c", 0): (1, 100.0, 100.0, 100.0)})
        self.assertIn("present_join is run-wide", self.refused("present_join", "lag_p50", "node"))
        msg = self.refused("nope", "lag_p50", "run")
        self.assertIn("no nope event; its kinds: first_read, join, latency, present_join, start", msg)
        # a kind both made would mix again
        self.derived(S, "derived_events", [["ks1", S, 1, "first_read", None]])
        self.assertIn("first_read is both a derived event and a sim's", self.refused("first_read", "lag_p50", "run"))

    def test_views_join_on_k_as_on_k_src(self):
        # kd's tables come from one source in both its cells: the fuller one
        self.assertEqual({(r["cell"], r["src"]) for r in self.sql("SELECT cell, src FROM runs WHERE k = 'kd'")},
                         {("d1", P), ("d2", P)})
        for using in ("k", "k, src"):
            got = self.sql(f"SELECT r.cell, count(*) AS n FROM reads JOIN runs r USING ({using}) "
                           "WHERE k = 'kd' GROUP BY ALL")
            self.assertEqual({r["cell"]: r["n"] for r in got}, {"d1": 50, "d2": 50}, using)
        # every view and macro joins runs on (k, src)
        v, have = simdb.views(self.args())
        made = {"sec": {"seconds", "verdict_seconds"}, "ev": {"derived_events"}, "read_bins(1000)": {"reads"},
                "run_read_bins(1000)": {"reads"}, "bins_by(1000)": {"bins"}}
        for t in [*(have - {"runs"}), *(m for m, needs in made.items() if needs <= have)]:
            simdb.duck(v + f"\nSELECT * FROM {t} JOIN runs USING (k, src) LIMIT 1;")
        # viewers name a node `node` in both sims, strims-sim's `host` kept
        for src, n in (([S], 2), ([P, S], 8)):
            got = self.sql("SELECT count(*) AS n FROM events JOIN viewers USING (k, src, node)", src)
            self.assertEqual(got[0]["n"], n, src)
        self.assertEqual({r["host"] for r in self.sql("SELECT host FROM viewers", [S])}, {3, 5})

    def test_sql_help_documents_the_joins(self):
        for argv in (["sql", "--help"], ["--help"]):
            out = io.StringIO()
            with (mock.patch.object(sys, "argv", ["simdb.py", *argv]), mock.patch.object(sys, "stdout", out),
                  self.assertRaises(SystemExit)):
                simdb.main()
            self.assertIn("JOIN runs USING (k, src)", out.getvalue(), argv)


# A farm run's flushes, (k, cell) each: i-3 holds a verify copy of k1, i-2
# a forecast log too, and i-4's samples never landed (its index says so,
# and S3 lacks them).
RUN = "sz0001-00+1more-zzzz"
C0, C1 = "sz0001-00", "sz0001-01"
FLUSHES = {"i-1": [("k1", C0), ("k2", C0)], "i-2": [("k3", C0), ("k4", C0), ("k5", C1)],
           "i-3": [("k1", C0), ("k6", C1)], "i-4": [("k7", C1)]}
FORECAST, UNLANDED = {"i-2"}, {"i-4"}


def src_of(inst):
    return f"farm/{RUN}/{inst}/0"


class SamplesPull(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        self.s3, self.calls = root / "s3", []
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(simdb, "derive", lambda srcs: None),
                  mock.patch.object(simdb, "aws", self.aws),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
            p.start()
            self.addCleanup(p.stop)
        (root / "hack" / "sim-farm").mkdir(parents=True)
        (root / "hack" / "sim-farm" / "config.toml").write_text('s3_bucket = "bkt"\ns3_region = "us-west-1"\n')
        farm = root / "target" / "sim-farm"
        (farm / RUN).mkdir(parents=True)
        (farm / RUN / "matrices").write_text(f"{C0}\n{C1}\n")
        index = {}
        q = simdb.q
        for inst, jobs in FLUSHES.items():
            src = src_of(inst)
            data, samples = self.s3 / "sim-data" / RUN / inst / "0", self.s3 / "sim-samples" / RUN / inst / "0"
            data.mkdir(parents=True)
            samples.mkdir(parents=True)
            runs = ", ".join(f"({q(k)}, {q(src)}, {q(c)}, 'main', {int(k[1:])}, '{{}}')" for k, c in jobs)
            reads = ", ".join(f"({q(k)}, {q(src)}, 0, {t}, {t // 100}, {int(k[1:]) * 10})"
                              for k, _ in jobs for t in range(0, int(k[1:]) * 1000, 100))
            tables = {"runs": (data, f"SELECT * FROM (VALUES {runs}) t(k, src, cell, arm, seed, env)", len(jobs)),
                      "reads": (samples, f"SELECT * FROM (VALUES {reads}) t(k, src, viewer, t_ms, seq, lag_ms)",
                                sum(int(k[1:]) * 10 for k, _ in jobs))}
            if inst in FORECAST:
                tables["forecast"] = (samples, f"SELECT k, src, 0 AS node, 1.5 AS sample_ms FROM (VALUES {runs}) "
                                               "t(k, src, cell, arm, seed, env)", len(jobs))
            manifest = {"keys": sorted(k for k, _ in jobs), "tables": {}}
            for t, (d, sql, n) in tables.items():
                simdb.duck(f"COPY ({sql}) TO {q(d / f'{t}.parquet')} (FORMAT parquet);", out=None)
                manifest["tables"][t] = {"rows": n, "bytes": (d / f"{t}.parquet").stat().st_size,
                                         "sha256": simdb.sha256(d / f"{t}.parquet")}
            (data / "manifest.json").write_text(json.dumps(manifest))
            if inst in UNLANDED:
                shutil.rmtree(samples)
            for k, c in jobs:
                index.setdefault(c, []).append({"key": k, "src": src, "arm": "main", "reads": inst not in UNLANDED,
                                                "copy": "v" if inst == "i-3" and k == "k1" else "", "row": {}})
        for m, lines in index.items():
            (farm / m).mkdir()
            (farm / m / "index.jsonl").write_text("".join(json.dumps(r) + "\n" for r in lines))

    def aws(self, args, region, profile):
        """The aws CLI over the scratch S3: `s3 sync` and `s3api get-object`."""
        self.assertEqual((region, profile), ("us-west-1", None))
        self.calls.append(args)
        if args[:2] == ["s3", "sync"]:
            prefix, dest, out = self.s3 / args[2].removeprefix("s3://bkt/"), Path(args[3]), []
            for f in sorted(prefix.rglob("*")):
                to = dest / f.relative_to(prefix)
                if f.is_file() and not to.exists():
                    to.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(f, to)
                    out.append(f"download: s3://bkt/{f.relative_to(self.s3)} to {to}\n")
            return subprocess.CompletedProcess(args, 0, "".join(out), "")
        if args[:2] == ["s3api", "get-object"]:
            f = self.s3 / args[args.index("--key") + 1]
            if not f.exists():
                return subprocess.CompletedProcess(args, 254, "", "An error occurred (NoSuchKey) when calling the "
                                                   "GetObject operation: The specified key does not exist.")
            shutil.copyfile(f, args[-1])
            return subprocess.CompletedProcess(args, 0, "{}", "")
        raise AssertionError(args)

    def gets(self):
        """The objects fetched one at a time, and forgets them."""
        got = {a[a.index("--key") + 1] for a in self.calls if a[:2] == ["s3api", "get-object"]}
        self.calls.clear()
        return got

    def size(self, inst, *tables):
        return sum((self.s3 / "sim-samples" / RUN / inst / "0" / f"{t}.parquet").stat().st_size for t in tables)

    def ledger(self):
        path = simdb.main_checkout() / "target" / "sim-farm" / "s3.jsonl"
        return [json.loads(line) for line in path.read_text().splitlines()] if path.exists() else []

    def cli(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with (mock.patch.object(sys, "argv", ["simdb.py", *argv]), mock.patch.object(sys, "stdout", out),
              mock.patch.object(sys, "stderr", err)):
            simdb.main()
        return out.getvalue(), err.getvalue()

    def args(self, cell, **kw):
        return argparse.Namespace(**{"src": [f"farm/{RUN}"], "cell": [cell], "arm": None, "seed": None, "bin": 1000,
                                     "since": None, "until": None, "json": True, "by": "none", **kw})

    def served(self, cell):
        """{k: (its reads' source's instance, reads)} in `cell`'s views."""
        v, _ = simdb.views(self.args(cell))
        return {r["k"]: (r["src"].split("/")[2], r["n"]) for r in simdb.duck(
            v + "\nSELECT k, src, count(*) AS n FROM reads GROUP BY ALL;")}

    def key(self, inst, t):
        return f"sim-samples/{RUN}/{inst}/0/{t}.parquet"

    def test_a_selection_pulls_the_fewest_flushes_holding_its_jobs(self):
        # The cache lacks the run: its core tables first (sim-data), then
        # the cell's samples from i-1 and i-2 alone, i-3's verify copy of
        # k1 left, and i-2's forecast with its reads.
        out, err = self.cli("pull", "--run", RUN, "--reads", "--cell", C0)
        self.assertEqual([a[:3] for a in self.calls if a[1] == "sync"], [["s3", "sync", f"s3://bkt/sim-data/{RUN}/"]])
        self.assertEqual(self.gets(), {self.key("i-1", "reads"), self.key("i-2", "reads"), self.key("i-2", "forecast")})
        self.assertIn("pulling the reads and forecast of 4 jobs the selection lacks: 2 flushes", err)
        sync, picked = self.ledger()
        self.assertEqual((sync["run"], sync["source"], sync["tier2"]["pull_get"]), (RUN, "pull", 8))
        self.assertEqual((picked["run"], picked["source"], picked["tier1"], picked["tier2"], picked["down"]),
                         (RUN, "pull", {}, {"pull_get": 3}, {"internet": self.size("i-1", "reads")
                                                             + self.size("i-2", "reads", "forecast")}))
        self.assertEqual(self.served(C0), {"k1": ("i-1", 10), "k2": ("i-1", 20), "k3": ("i-2", 30),
                                           "k4": ("i-2", 40)})
        # Pulled again: nothing to fetch.
        self.cli("pull", "--run", RUN, "--reads", "--cell", C0)
        out, _ = self.cli("pull", "--run", RUN, "--reads", "--cell", C0, "--dry-run")
        self.assertIn("nothing to pull", out)
        self.assertEqual((self.gets(), len(self.ledger())), (set(), 2))
        # The other cell: k5 is in place (i-2), k6 is i-3's, and k7's never
        # landed. A dry run asks S3 nothing.
        out, _ = self.cli("pull", "--run", RUN, "--reads", "--cell", C1, "--dry-run")
        self.assertIn(f"would GET 1 files, {self.size('i-3', 'reads') / 1e6:.1f} MB, for 1 jobs' reads and forecast; "
                      "S3 holds none for 1 jobs", out)
        self.assertIn(f"s3://bkt/{self.key('i-3', 'reads')}", out)
        self.assertEqual((self.calls, len(self.ledger())), ([], 2))
        # A query pulls what it reads of its selection, noting the size,
        # within --pull-mb.
        err = io.StringIO()
        with (mock.patch.object(sys, "stdout", io.StringIO()), mock.patch.object(sys, "stderr", err),
              self.assertRaises(SystemExit) as cm):
            simdb.cmd_pooled(self.args(C1, pull_mb=0.0001))
        self.assertIn("past --pull-mb", str(cm.exception.code))
        self.assertEqual(self.calls, [])
        out = io.StringIO()
        with mock.patch.object(sys, "stdout", out), mock.patch.object(sys, "stderr", err):
            simdb.cmd_pooled(self.args(C1, pull_mb=simdb.PULL_MB))
        self.assertEqual({(r["cell"], r["runs"], r["reads"]) for r in json.loads(out.getvalue())}, {(C1, 2, 110)})
        self.assertEqual(self.gets(), {self.key("i-3", "reads")})
        self.assertIn(f"pulling the reads of 1 jobs the selection lacks: 1 flushes, "
                      f"{self.size('i-3', 'reads') / 1e6:.1f} MB", err.getvalue())
        self.assertIn("S3 holds none of the reads of 1 of the selection's jobs", err.getvalue())
        self.assertEqual(self.ledger()[-1]["tier2"], {"pull_get": 1})
        self.assertEqual(self.ledger()[-1]["down"], {"internet": self.size("i-3", "reads")})

    def test_a_flush_gone_from_s3_gives_way_to_a_copy(self):
        self.cli("pull", "--run", RUN)
        self.calls.clear()
        (self.s3 / self.key("i-1", "reads")).unlink()
        _, err = self.cli("pull", "--run", RUN, "--reads", "--cell", C0)
        self.assertEqual(self.gets(), {self.key("i-1", "reads"), self.key("i-2", "reads"), self.key("i-2", "forecast"),
                                       self.key("i-3", "reads")})
        self.assertIn("S3 holds none of the reads or forecast of 1 of the selection's jobs", err)
        # the missing key's GET counts too
        self.assertEqual(self.ledger()[-1]["tier2"], {"pull_get": 4})
        self.assertEqual(self.ledger()[-1]["down"],
                         {"internet": self.size("i-2", "reads", "forecast") + self.size("i-3", "reads")})
        self.assertEqual(self.served(C0), {"k1": ("i-3", 10), "k3": ("i-2", 30), "k4": ("i-2", 40)})

    def test_a_whole_run_pull_still_syncs_every_flush(self):
        self.cli("pull", "--run", RUN)
        out, _ = self.cli("pull", "--run", RUN, "--reads", "--dry-run")
        self.assertIn("would GET 4 files", out)
        self.calls.clear()
        self.cli("pull", "--run", RUN, "--reads")
        self.assertEqual([a[2] for a in self.calls], [f"s3://bkt/sim-data/{RUN}/", f"s3://bkt/sim-samples/{RUN}/"])
        cache = simdb.cache_root() / "cache"
        self.assertEqual({p.relative_to(cache).as_posix() for p in cache.glob("farm/**/*.parquet")
                          if p.stem in simdb.SAMPLE_TABLES},
                         {f"{src_of(i)}/{t}.parquet" for i, t in (("i-1", "reads"), ("i-2", "reads"),
                                                                  ("i-2", "forecast"), ("i-3", "reads"))})
        self.assertEqual(self.ledger()[-1]["tier2"], {"pull_get": 4})
        self.assertEqual(self.ledger()[-1]["down"],
                         {"internet": sum(self.size(i, "reads") for i in ("i-1", "i-2", "i-3"))
                          + self.size("i-2", "forecast")})


if __name__ == "__main__":
    unittest.main()
