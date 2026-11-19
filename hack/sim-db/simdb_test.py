#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's selections over a scratch cache, suite indexes and farm state,
as two farm runs of several suite tiers leave them (an A/B and its
confirmation, each a browsers and a net tier, their cells sharing names,
the A/B reusing rows of the confirmation's); the arms simdb and suite.py
name a suite farm run's jobs by, alike, and the index suite.py's `farm`
writes (ArmLabels); which copy of a key the views take (Canon); its
queries and joins over a scratch cache of both sims' tables (Queries);
its pulls of a farm run's samples from a scratch S3, the aws CLI stubbed
(SamplesPull), and of one run of a name S3 holds two runs of, by commit
and time, with the prune of the other's flushes (OneRun); the derived
tables' columns and versions (Derived); the events load_series gives the phase rule, retired ids read as
current ones and a source without events implying them (RuleEvents);
macros.sql's playback() against the sims' own stalls (Playback); and
catalog.py's index of a scratch cache and farm state (Catalog):

  python3 hack/sim-db/simdb_test.py

Needs the pinned DuckDB CLI (simdb.py duckdb fetches it); runs under
hack/'s uv venv (the catalog's in the duckdb module)."""

import argparse
import datetime
import fnmatch
import importlib.util
import io
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

spec = importlib.util.spec_from_file_location("simdb", Path(__file__).with_name("simdb.py"))
simdb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(simdb)
# simlab's simdb (simlab.simdb()) is this one, so the tests' patches reach
# simlab.store's pulls too.
sys.modules["simdb"] = simdb
sys.path.insert(0, str(Path(__file__).parents[1]))
from simlab import bootstrap, query, sketch, store, store_host, verdict  # noqa: E402

spec = importlib.util.spec_from_file_location("suite", Path(__file__).parents[1] / "sim-suite" / "suite.py")
suite = importlib.util.module_from_spec(spec)
spec.loader.exec_module(suite)
# suite.py's runs and farm matrices are simlab.suite's (suite.lab), whose
# simdb is this one, so the tests' patches reach it.
suite.lab._simdb = simdb
spec = importlib.util.spec_from_file_location("catalog", Path(__file__).with_name("catalog.py"))
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)



def checkout(root):
    """Patches making `root` the main checkout: store_host's, which the
    selections and pulls read, and simdb's, which the catalog reads."""
    return (mock.patch.object(store_host, "main_checkout", lambda: root),
            mock.patch.object(simdb, "main_checkout", lambda: root))


def views(a, derived=True):
    """The SQL of the views of simdb.py's options `a`, and their tables."""
    v = query.views(query.Selection.of(a), derived)
    return v.sql, v.have


A, B = "farm/sa0001-00+3more-aaaa", "farm/sb0001-00+3more-bbbb"
GONE = "farm/sc0001-00/i-1/0"
TIER = {"cells": [{"name": "star-3-5", "sim": "strims-sim"}, {"name": "halve-7-10", "sim": "strims-sim"},
                  {"name": "star-7-5", "sim": "strims-sim"}]}
INDEXES = {
    "sa0001": TIER,
    "sa0002": {"cells": [{"name": "org-star-7-5", "sim": "strims-sim"}],
               "knob": {"strims-sim": "STRIMS_SIM_EXP"}, "values": ["off", "a"]},
    "sb0001": TIER,
}
# Kept apart by suite.py until its run started.
PENDING = {"sb0002": {"cells": [{"name": "org-star-7-5", "sim": "strims-sim"}]}}
MATRICES = {A: ["sa0001-00", "sa0001-01", "sa0001-02", "sa0002-00"],
            B: ["sb0001-00", "sb0001-01", "sb0001-02", "sb0002-00"]}
# source: (k, matrix, arm, env)
STORED = {
    f"{A}/i-1/0": [("k1", "sa0001-00", "main", {}), ("k2", "sa0001-00", "fix", {}),
                   ("k3", "sa0001-01", "main", {}), ("k4", "sa0002-00", "x", {"STRIMS_SIM_EXP": "off"}),
                   ("k5", "sa0002-00", "x", {"STRIMS_SIM_EXP": "a"}),
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
        for p in (*checkout(root),
                  mock.patch.object(query, "pull_sources", lambda s, reads=False: self.pulled.extend(s) or list(s)),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
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
        v, _ = views(argparse.Namespace(src=src, cell=cell, arm=None, seed=None), derived=False)
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

    def test_arms_sharing_a_key_each_hold_its_row(self):
        # Arm main of sa0001-00 shares k1 with arm fix there, their binaries
        # the same bytes: the farm ran it once, under main, whose flush
        # stores it, and fix's line names its own matrix (`from`).
        index = simdb.main_checkout() / "target" / "sim-farm" / "sa0001-00" / "index.jsonl"
        with index.open("a") as f:
            f.write("\n" + json.dumps({"key": "k1", "arm": "fix", "src": f"{A}/i-1/0", "from": "sa0001-00",
                                       "row": {}}) + "\n")
        for src in (None, [f"{A}*"], [f"{A}/i-1/0"]):
            got = self.select(src, ["star-3-5@sa0001"], "SELECT k, arm, count(*) AS n FROM runs GROUP BY ALL")
            self.assertEqual({(r["k"], r["arm"]): r["n"] for r in got},
                             {("k1", "main"): 1, ("k1", "fix"): 1, ("k2", "fix"): 1}, src)
            v, _ = views(argparse.Namespace(src=src, cell=None, arm=["fix"], seed=None), derived=False)
            got = simdb.duck(v + "\nSELECT k, src FROM runs WHERE cell = 'star-3-5@sa0001' ORDER BY k;")
            self.assertEqual([r["k"] for r in got], ["k1", "k2"], src)
            self.assertEqual(got[0]["src"], f"{A}/i-1/0", src)

    def test_a_key_two_jobs_ran_into_one_flush_holds_each_jobs_row(self):
        # Before the farm ran a shared key once, arms main and fix2 of
        # sa0001-00 each ran k1 (their binaries the same bytes) and both
        # copies went into one flush, which keeps main's: fix2's line has no
        # `from`, its row the one main's is.
        index = simdb.main_checkout() / "target" / "sim-farm" / "sa0001-00" / "index.jsonl"
        with index.open("a") as f:
            f.write("\n" + json.dumps({"key": "k1", "arm": "fix2", "src": f"{A}/i-1/0", "row": {}}) + "\n")
        for src in (None, [f"{A}*"], [f"{A}/i-1/0"]):
            got = self.select(src, ["star-3-5@sa0001"], "SELECT k, arm, count(*) AS n FROM runs GROUP BY ALL")
            self.assertEqual({(r["k"], r["arm"]): r["n"] for r in got},
                             {("k1", "main"): 1, ("k1", "fix2"): 1, ("k2", "fix"): 1}, src)
        # A selection without that flush adds nothing of it.
        got =self.select([f"{A}/i-2/0"], None, "SELECT k, arm FROM runs WHERE cell = 'star-3-5@sa0001'")
        self.assertEqual([(r["k"], r["arm"]) for r in got], [("k2", "fix")])

    def test_rows_prints_a_key_in_each_of_its_cells(self):
        # kr and kq: B's rows, which A's run reused; rows() gives a key's
        # row once, whatever cells it counts in.
        verdict = {"healthy": [True] * 30, "onset": None, "recovered": None, "absorbing": False,
                   "unhealthy": 0.0, "lag_p50": 200.0}
        rows = lambda pairs: {p: {"lag_p99_ms": 300, "_verdict": verdict, "_metrics": {}} for p in pairs}
        out = io.StringIO()
        with mock.patch.object(query, "rows", rows), mock.patch.object(sys, "stdout", out):
            simdb.cmd_rows(argparse.Namespace(src=None, cell=["halve-7-10@*"], arm=None, seed=None, fields=None))
        got = [json.loads(line) for line in out.getvalue().splitlines()]
        self.assertEqual(sorted((r["cell"], r["k"]) for r in got if r["k"] in ("kr", "kq")),
                         [("halve-7-10@sa0001", "kq"), ("halve-7-10@sa0001", "kr"), ("halve-7-10@sb0001", "kq"),
                          ("halve-7-10@sb0001", "kr")])
        for r in got:
            self.assertEqual((r["lag_p99_ms"], r["_metrics"], r["_collapsed"]), (300, {}, False), r)
            self.assertEqual(r["_verdict"], {x: verdict[x] for x in ("onset", "recovered", "absorbing", "unhealthy",
                                                                     "lag_p50")}, r)


# Suite farm runs, as suite.py's `farm` leaves them: a --sets run (the
# knob its index names, apart from the default STRIMS_SIM_EXP), a --refs
# run whose --env sets the default knob, one ref named as a set ("off"),
# and two runs without an index: one of sets (the matrices' lone arm, x,
# the default knob an axis), and one of refs whose env gives each the
# other's name.
LABEL_CELLS = [{"name": "star-3-5", "sim": "strims-sim", "test": "calibration_star"},
               {"name": "org-star-7-5", "sim": "strims-sim", "test": "calibration_star"}]
SETS_KNOB = {"strims-sim": "STRIMS_SIM_KNOWN_UPLINK"}
LABEL_INDEXES = {"sd0001": {"cells": LABEL_CELLS, "knob": SETS_KNOB, "values": ["off", "seed", "1"]},
                 "sd0002": {"cells": LABEL_CELLS}}
# run: [(arm, {sim: (knob, value)})]
LABEL_ARMS = {
    "sd0001": [("x", {"strims-sim": ("STRIMS_SIM_KNOWN_UPLINK", v)}) for v in ("off", "seed", "1")],
    "sd0002": [(r, {"strims-sim": ("STRIMS_SIM_EXP", "off")}) for r in ("main", "off")],
    "sd0003": [("x", {"strims-sim": ("STRIMS_SIM_EXP", v)}) for v in ("off", "a")],
    "sd0004": [(r, {"strims-sim": ("STRIMS_SIM_EXP", v)}) for r, v in (("off", "on"), ("on", "off"))],
}
LABEL_SEEDS = (1, 2)


class ArmLabels(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = root = Path(tmp.name)
        for p in (*checkout(root),
                  mock.patch.object(suite.lab, "farm_root", lambda: root),
                  mock.patch.object(query, "pull_sources", lambda s, reads=False: []),
                  mock.patch.object(suite.lab, "index_catalog", lambda: None),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        runs = root / "target" / "sim-suite" / "farm-runs"
        runs.mkdir(parents=True)
        for run, idx in LABEL_INDEXES.items():
            (runs / f"{run}.json").write_text(json.dumps(idx))
        # key: (run, cell, arm, knob value, seed)
        self.jobs = {}
        for run, arms in LABEL_ARMS.items():
            src = f"farm/{run}-00+1more-cccc/i-1/0"
            stored = []
            for i, c in enumerate(LABEL_CELLS):
                lines = []
                for arm, env in arms:
                    knob, value = env[c["sim"]]
                    for s in LABEL_SEEDS:
                        k = f"{run}-{i}-{arm}-{value}-{s}"
                        self.jobs[k] = (run, c["name"], arm, value, s)
                        e = {knob: value, "STRIMS_SIM_SEED": str(s)}
                        lines.append({"id": k, "key": k, "arm": arm, "test": c["test"], "env": e, "src": src,
                                      "row": {}})
                        stored.append((k, f"{run}-{i:02d}", arm, s, e))
                d = root / "target" / "sim-farm" / f"{run}-{i:02d}"
                d.mkdir(parents=True)
                (d / "index.jsonl").write_text("".join(json.dumps(r) + "\n" for r in lines))
            q = simdb.q
            d = simdb.cache_root() / "cache" / src
            d.mkdir(parents=True)
            vals = ", ".join(f"({q(k)}, {q(src)}, {q(m)}, {q(arm)}, {s}, {q(json.dumps(env))})"
                             for k, m, arm, s, env in stored)
            simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, cell, arm, seed, env)) "
                       f"TO {q(d / 'runs.parquet')} (FORMAT parquet);", out=None)

    def farm_rows(self, run):
        """{arm: {(cell, seed): key}}"""
        rows = suite.farm_rows(run, LABEL_CELLS, LABEL_INDEXES[run].get("knob"))
        return {arm: {at: k for at, (k, _) in rs.items()} for arm, rs in rows.items()}

    def score(self, arg, knob=None):
        """`score`'s rows of RUN:ARM `arg`, {(cell, seed): key}."""
        got = {}
        args = argparse.Namespace(a=arg, b=arg, knob=knob, only=None, rank="lag", all=False)

        def scorecard(cells, a_rows, b_rows, *_):
            got.update(a_rows)
            return mock.Mock(text=lambda: "")

        with mock.patch.object(suite.scoring, "scorecard", scorecard):
            suite.score(args, LABEL_CELLS)
        return {at: k for at, (k, _) in got.items()}

    def want(self, run, pick):
        """{(cell, seed): key} of `run`'s jobs `pick(arm, value)` takes."""
        return {(c, s): k for k, (r, c, arm, v, s) in self.jobs.items() if r == run and pick(arm, v)}

    def test_a_sets_run_scores_each_set_apart(self):
        got = self.farm_rows("sd0001")
        self.assertEqual(set(got), {"off", "seed", "1"})
        for v in ("off", "seed", "1"):
            self.assertEqual(got[v], self.want("sd0001", lambda arm, x: x == v), v)
            self.assertEqual(self.score(f"sd0001:{v}"), got[v], v)
        with self.assertRaises(SystemExit) as cm:
            self.score("sd0001:x")
        self.assertIn("its arms are sets: 1, off, seed", str(cm.exception.code))
        with self.assertRaises(SystemExit) as cm:
            self.score("sd0001:off", knob="STRIMS_SIM_EXP")
        self.assertIn("index's knob", str(cm.exception.code))
        self.assertEqual(self.score("sd0001:off", knob="STRIMS_SIM_KNOWN_UPLINK"), got["off"])

    def test_a_refs_run_is_named_by_its_refs_alone(self):
        # Its env's knob value names no arm, and its ref named "off" holds
        # its own rows alone.
        got = self.farm_rows("sd0002")
        self.assertEqual(set(got), {"main", "off"})
        for r in ("main", "off"):
            self.assertEqual(got[r], self.want("sd0002", lambda arm, _: arm == r), r)
            self.assertEqual(self.score(f"sd0002:{r}"), got[r], r)

    def test_a_run_without_an_index_is_read_both_ways(self):
        self.assertEqual(self.score("sd0003:a"), self.want("sd0003", lambda _, v: v == "a"))
        self.assertEqual(self.score("sd0003:off"), self.want("sd0003", lambda _, v: v == "off"))
        # x holds every set's jobs at a cell and seed: no arm.
        with self.assertRaises(SystemExit) as cm:
            self.score("sd0003:x")
        self.assertIn("no rows of arm 'x' (it has a, off)", str(cm.exception.code))
        # Each ref's env names the other: either name is ambiguous.
        for r in ("off", "on"):
            with self.assertRaises(SystemExit) as cm:
                self.score(f"sd0004:{r}")
            self.assertIn("names both a set", str(cm.exception.code))

    def test_the_index_describes_what_ran(self):
        # `farm --no-run`'s index, kept apart until the run starts: each
        # cell as its matrix runs it, the arms, the seeds and the sets. The
        # fixture's own indexes, cells without env, load as ever (above).
        cells = [{**suite.cell("star-3-5", suite.SIM, "calibration_star", {"STRIMS_SIM_CAL_VIEWERS": "3"}, 2,
                               "3 viewers"), "seeds": 12},
                 suite.cell("large", suite.SIM, "calibration_star", {"STRIMS_SIM_CAL_VIEWERS": "200"}, 3,
                            "a large swarm")]
        args = argparse.Namespace(refs=None, sets="seed;1", ref="HEAD", no_off=False, knob="STRIMS_SIM_EXP",
                                  first_seed=2001, seeds=3, env=[], no_run=True, farm_args="", against=None,
                                  question="does seed help?", all=False, rank="lag")
        sha = "a" * 40
        # The farm holds no rows of the run: its scores print nothing.
        silent = lambda *a, **kw: mock.Mock(text=lambda: "")  # noqa: E731
        with mock.patch.object(suite.lab, "git", lambda *a, **kw: sha), \
                mock.patch.object(suite.lab, "peak_gib", lambda r: {"star-3-5": 0.5}), \
                mock.patch.object(suite.lab, "WORK", self.root / "work"), \
                mock.patch.object(suite.scoring, "arm_score", silent), \
                mock.patch.object(suite.scoring, "scorecard", silent):
            suite.farm(args, cells)
        [path] = (self.root / "target" / "sim-suite" / "farm-runs" / "pending").glob("*.json")
        idx = simdb.suite_index(path.stem)
        self.assertEqual(idx["cells"], [
            {"name": "star-3-5", "sim": "strims-sim", "test": "calibration_star", "why": "3 viewers",
             "env": {"STRIMS_SIM_CAL_VIEWERS": "3"}, "mem_gib": 1.0},
            {"name": "large", "sim": "strims-sim", "test": "calibration_star", "why": "a large swarm",
             "env": {"STRIMS_SIM_CAL_VIEWERS": "200"}, "mem_gib": 3}])
        self.assertEqual((idx["arms"], idx["seeds"], idx["knob"], idx["values"]),
                         ({"x": sha}, [2001, 2002, 2003], {"strims-sim": "STRIMS_SIM_EXP"}, ["off", "seed", "1"]))
        self.assertEqual((idx["base"], idx["question"]), ("off", "does seed help?"))
        self.assertEqual(simdb.suite_cells([f"{path.stem}-01"]),
                         {f"{path.stem}-01": (f"large@{path.stem}", "STRIMS_SIM_EXP")})

    def test_a_label_holding_two_jobs_is_refused(self):
        with self.assertRaises(SystemExit) as cm:
            suite.farm_rows("sd0001", LABEL_CELLS, None)
        self.assertIn("arm x holds two jobs at one cell and seed", str(cm.exception.code))

    def test_simdb_names_the_arms_as_suite_does(self):
        v, _ = views(argparse.Namespace(src=None, cell=None, arm=None, seed=None), derived=False)
        named = {r["k"]: (r["cell"], r["arm"]) for r in simdb.duck(v + "\nSELECT k, cell, arm FROM runs;")}
        for run in LABEL_INDEXES:
            for arm, rows in self.farm_rows(run).items():
                for (cell, _), k in rows.items():
                    self.assertEqual(named[k], (f"{cell}@{run}", arm), k)
        self.assertEqual(simdb.arm_label(SETS_KNOB, "strims-sim", "x", {"STRIMS_SIM_KNOWN_UPLINK": "1"}), "1")
        self.assertEqual(simdb.arm_label(None, "strims-sim", "main", {"STRIMS_SIM_EXP": "off"}), "main")


class Canon(unittest.TestCase):
    """Which of a key's copies the views take its tables from: the one
    holding the most tables, then the later flush, then the later seq."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (*checkout(root),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)

    def put(self, src, tables=("runs", "seconds"), at=None):
        d = simdb.cache_root() / "cache" / src
        d.mkdir(parents=True)
        for t in tables:
            simdb.duck(f"COPY (SELECT 'kt' AS k, {simdb.q(src)} AS src, 'c' AS cell, 'a' AS arm, 1 AS seed, 0 AS sec) "
                       f"TO {simdb.q(d / f'{t}.parquet')} (FORMAT parquet);", out=None)
        if at:
            (d / "manifest.json").write_text(json.dumps({"keys": ["kt"], "at": at}))

    def canon(self):
        v, _ = views(argparse.Namespace(src=None, cell=None, arm=None, seed=None), derived=False)
        got = simdb.duck(v + "\nSELECT (SELECT src FROM canon) AS canon, (SELECT src FROM runs) AS runs;")[0]
        self.assertEqual(got["canon"], got["runs"])
        return got["canon"]

    def test_seq_ranks_by_number(self):
        run = "farm/sx0001-00/i-1"
        for seq in (2, 9, 10):
            self.put(f"{run}/{seq}")
        self.assertEqual(self.canon(), f"{run}/10")

    def test_the_later_flush_ranks_first(self):
        self.put("farm/sx0001-00/i-1/10", at="2026-11-10T05:00:00Z")
        self.put("farm/sx0001-00/i-2/3", at="2026-11-10T06:00:00Z")
        self.put("farm/sx0001-00/i-3/9", at="2026-11-10T04:00:00Z")
        self.assertEqual(self.canon(), "farm/sx0001-00/i-2/3")

    def test_the_most_tables_rank_first(self):
        self.put("farm/sx0001-00/i-1/10", at="2026-11-10T05:00:00Z")
        self.put("farm/sx0001-00/i-1/9", ("runs", "seconds", "viewers"), at="2026-11-10T04:00:00Z")
        self.assertEqual(self.canon(), "farm/sx0001-00/i-1/9")


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
# (k, t_ms, kind): an event's detail, else empty
DETAILS = {("kp1", 3000, "first_read"): "late"}
# kp1's forecast log: (t_ms, node, viewer, peer, peer_honest, live, run,
# first_sight, mean_ms, held_ms, seen_ms, sample_ms); the error held -
# (seen + mean) is node 1's 50 and -120, node 2's 90, the rest left out (a
# hostile peer, an arrival the sim didn't see, a first sight, a second run)
FORECASTS = [(1200, 1, 0, 2, True, True, 0, False, 50, 1100, 1000, 60),
            (1500, 1, 0, 2, True, True, 0, False, 20, 1300, 1400, 30),
            (1600, 1, 0, 0, False, True, 0, False, 10, 1500, 1500, 10),
            (1700, 1, 0, 2, True, False, 0, False, 10, 1500, 1500, 10),
            (1800, 1, 0, 2, True, True, 0, True, 10, 1500, 1500, 10),
            (1900, 1, 0, 2, True, True, 1, False, 10, 1500, 1500, 10),
            (3500, 2, 1, 1, True, True, 0, False, 10, 3600, 3500, 20)]
# k: (viewer, first read's t_ms, lag_ms): a read every 100 ms to 4900
READ_RUNS = {"kp1": [(0, 1000, 100), (1, 3000, 300)], "kp2": [(0, 2000, 200)], "kd": [(0, 0, 50)],
             "ks1": [(0, 1500, 150), (1, 2500, 250)]}
READS = {k: [(v, t, lag) for v, start, lag in rs for t in range(start, 5000, 100)] for k, rs in READ_RUNS.items()}
# (k, sec): the run's phase (verdict_seconds)
PHASES = {**{("kp1", s): "startup" for s in range(3)}, **{("kp1", s): "steady" for s in (3, 4)},
          **{("kp2", s): "startup" for s in range(5)}}
# (k, viewer, sec): a viewer's phase (viewer_phases), the run's but kp1's
# viewer 1's, in its own startup from its first read at 3 s; a second
# without one is the tail's
VIEWER_PHASES = {**{(k, v, s): ph for (k, s), ph in PHASES.items() for v, _, _ in READ_RUNS[k]},
                 ("kp1", 1, 3): "startup", ("kp1", 1, 4): "startup"}
# k: the viewers present each second (verdict.events' present_join and
# present_leave): kp1's second viewer counts from 2 s
PRESENT = {"kp1": [1, 1, 2, 2, 2], "kp2": [1] * 5, "kd": [1] * 5, "ks1": [2] * 5}


def qc(xs, p):
    """sketch_quantile's p-th percentile (simlab.sketch's)."""
    return sketch.quantile(xs, p)


class Queries(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (*checkout(root),
                  mock.patch.object(query, "derive", lambda srcs: None),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        for src, rows in RUNS.items():
            self.put(src, "runs", ["cell", "arm", "seed", "env", "bin_ms"],
                     [(k, cell, arm, seed, "{}", 100) for k, cell, arm, seed in rows])
            keys = [r[0] for r in rows]
            self.put(src, "reads", ["viewer", "t_ms", "seq", "lag_ms"],
                     [(k, v, t, t // 100, lag) for k in keys for v, t, lag in READS[k]])
            self.derived(src, "derived_events", [
                [k, src, t, kind, value] for k in keys for t, kind, value in verdict.events(
                    {"present": PRESENT[k], "seed_up": [(0, 1000)]},
                    {"healthy": [True] * 5, "onset": None, "recovered": None})])
        self.put(P, "viewers", ["viewer", "node"], [(k, v, n) for k in ("kp1", "kp2", "kd") for v, n in VIEWERS[k]])
        self.put(S, "viewers", ["viewer", "host"], [("ks1", v, n) for v, n in VIEWERS["ks1"]])
        for src, keys in ((P, ("kp1", "kp2")), (S, ("ks1",))):
            self.put(src, "events", ["t_us", "kind", "node", "peer", "value", "detail"],
                     [(k, t * 1000, kind, n, -1, 0, DETAILS.get((k, t, kind), ""))
                      for k in keys for t, kind, n in EVENTS[k]])
        self.put(P, "forecast", ["t_ms", "node", "viewer", "peer", "peer_honest", "live", "run", "first_sight",
                                 "mean_ms", "held_ms", "seen_ms", "sample_ms"], [("kp1", *f) for f in FORECASTS])
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
        self.derived(P, "viewer_phases", [[k, P, v, ph, s, s + 1] for (k, v, s), ph in VIEWER_PHASES.items()])

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
                                     "since": None, "until": None, "json": True, "detail": None, "pool": False,
                                     **kw})

    def run_cmd(self, cmd, **kw):
        out = io.StringIO()
        with mock.patch.object(sys, "stdout", out):
            cmd(self.args(**kw))
        return json.loads(out.getvalue())

    def sql(self, query, src=None):
        v, _ = views(self.args(src=src))
        return simdb.duck(v + "\n" + query + ";")

    def aligned(self, event, field, per, nth=1, before=1, after=2, node=None, **kw):
        got = self.run_cmd(simdb.cmd_aligned, event=event, field=field, per=per, nth=nth, before=before,
                           after=after, node=node, **kw)
        return {(r["cell"], r["rel_ms"]): (r["n"], r["p10"], r["p50"], r["p90"]) for r in got}

    def refused(self, event, field, per, **kw):
        with self.assertRaises(SystemExit) as cm:
            self.run_cmd(simdb.cmd_aligned, event=event, field=field, per=per, nth=1, before=1, after=2, node=None,
                         **kw)
        return str(cm.exception.code)

    def test_pooled_columns_are_each_groups(self):
        cells = {}
        for rows in RUNS.values():
            for k, cell, arm, _ in rows:
                cells.setdefault((cell, arm), set()).add(k)
        group = {"none": lambda k, v, t: None, "bin": lambda k, v, t: t // 1000 * 1000,
                 "viewer": lambda k, v, t: v, "phase": lambda k, v, t: VIEWER_PHASES.get((k, v, t // 1000), "tail")}
        for by, g in group.items():
            want = {}
            for (cell, arm), keys in cells.items():
                for k in keys:
                    for v, t, lag in READS[k]:
                        want.setdefault((cell, arm, g(k, v, t)), {}).setdefault(k, []).append(lag)
            want = {key: (len(runs), sum(map(len, runs.values())),
                          sum(qc(x, 99) for x in runs.values()) / len(runs)) for key, runs in want.items()}
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
        # every node of cell c reads its own lag from its own first read on:
        # 100, 150, 200, 250 and 300, whose p10, p50 and p90 are the 1st,
        # 3rd and 5th (the integer nearest rank)
        self.assertEqual(self.aligned("first_read", "lag_p50", "node"),
                         {("c", 0): (5, 100.0, 200.0, 300.0), ("c", 1000): (5, 100.0, 200.0, 300.0)})
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
        # (two nodes, 100 and 300 ms: the p10 and p50 the first, the p90
        # the second)
        self.assertEqual(self.aligned("first_read", "lag_ms", "node", after=1),
                         {("c", 0): (2, 100.0, 100.0, 300.0)})
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
        # every kind verdict.events makes: a viewer joins at 5 s and one
        # leaves at 15, the seed's uplink halves at 20 s and comes back at
        # 30, and 16-25 s are unhealthy, a collapse
        healthy = [True] * 16 + [False] * 10 + [True] * 14
        got = verdict.events({"present": [3] * 5 + [5] * 10 + [4] * 25,
                                      "seed_up": [(0, 1000), (20, 500), (30, 1000)]},
                                     {"healthy": healthy, "onset": 16, "recovered": 26})
        self.assertEqual({kind for _, kind, _ in got}, set(simdb.DERIVED_KINDS))
        self.assertIn((5, "present_join", 2), got)
        self.assertIn((15, "present_leave", 1), got)
        # a run's nth join is the sims' (kp1's node 2 again at 4 s: ten
        # reads of 100 and ten of 300 ms, whose p50 is the 10th), not the
        # viewers present rising (kp1 at 2 s)
        self.assertEqual(self.aligned("join", "lag_p50", "run", nth=3, before=0, after=1),
                         {("c", 0): (1, 100.0, 100.0, 100.0)})
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
        v, have = views(self.args())
        made = {"sec": {"seconds", "verdict_seconds"}, "ev": {"derived_events"}, "read_bins(1000)": {"reads"},
                "run_read_bins(1000)": {"reads"}, "bins_by(1000)": {"bins"}}
        for t in [*(have - {"runs"}), *(m for m, needs in made.items() if needs <= have)]:
            simdb.duck(v + f"\nSELECT * FROM {t} JOIN runs USING (k, src) LIMIT 1;")
        # viewers name a node `node` in both sims, strims-sim's `host` kept
        for src, n in (([S], 2), ([P, S], 8)):
            got = self.sql("SELECT count(*) AS n FROM events JOIN viewers USING (k, src, node)", src)
            self.assertEqual(got[0]["n"], n, src)
        self.assertEqual({r["host"] for r in self.sql("SELECT host FROM viewers", [S])}, {3, 5})

    def test_aligned_anchors_on_the_departures(self):
        # kp1's viewers leave at 2.5 s (node 1, gracefully) and 3.5 s (node
        # 2, abruptly): a run's first departure opens the window, each
        # node's its own, and --detail picks a kind.
        departs = [("kp1", 2500, "depart", 1, "graceful"), ("kp1", 3500, "depart", 2, "abrupt")]
        self.put(P, "events", ["t_us", "kind", "node", "peer", "value", "detail"],
                 [(k, t * 1000, kind, n, -1, 0, DETAILS.get((k, t, kind), ""))
                  for k in ("kp1", "kp2") for t, kind, n in EVENTS[k]]
                 + [(k, t * 1000, kind, n, -1, 0, d) for k, t, kind, n, d in departs])
        # the 1 s after 2.5 s: viewer 0's ten reads and viewer 1's first
        # five; after 3.5 s, ten each
        self.assertEqual(self.aligned("depart", "reads", "run", before=0, after=1), {("c", 0): (1, 15.0, 15.0, 15.0)})
        # of two (15 and 20), the p10 and p50 are the first, the p90 the
        # second
        self.assertEqual(self.aligned("depart", "reads", "run", nth=0, before=0, after=1),
                         {("c", 0): (2, 15.0, 15.0, 20.0)})
        self.assertEqual(self.aligned("depart", "lag_p50", "node", before=0, after=1),
                         {("c", 0): (2, 100.0, 100.0, 300.0)})
        # the run's reads after 3.5 s, ten of 100 and ten of 300 ms: the
        # p50 is the 10th
        self.assertEqual(self.aligned("depart", "lag_p50", "run", before=0, after=1, detail="abrupt"),
                         {("c", 0): (1, 100.0, 100.0, 100.0)})

    def test_aligned_takes_every_occurrence_and_picks_events(self):
        # cell c's first reads, a window each: kp1's at 1 s (its first
        # viewer's ten reads in the second after) and at 3 s (both viewers'
        # twenty), kp2's at 2 s (ten), ks1's at 1.5 s (ten) and 2.5 s (twenty)
        self.assertEqual(self.aligned("first_read", "reads", "run", nth=0, before=0, after=1),
                         {("c", 0): (5, 10.0, 10.0, 20.0)})
        self.assertEqual(self.aligned("first_read", "reads", "run", before=0, after=1),
                         {("c", 0): (3, 10.0, 10.0, 10.0)})
        # --since and --until pick the events by time: a run's first from 2 s
        # on, and every one before 2 s
        self.assertEqual(self.aligned("first_read", "reads", "run", before=0, after=1, since=2),
                         {("c", 0): (3, 10.0, 20.0, 20.0)})
        self.assertEqual(self.aligned("first_read", "reads", "run", nth=0, before=0, after=1, until=2),
                         {("c", 0): (2, 10.0, 10.0, 10.0)})
        # --detail by their detail (a glob): kp1's late one alone
        self.assertEqual(self.aligned("first_read", "reads", "run", before=0, after=1, detail="la*"),
                         {("c", 0): (1, 20.0, 20.0, 20.0)})
        self.assertEqual(self.aligned("first_read", "reads", "run", before=0, after=1, detail="nope"), {})
        # per node, each node's own: kp1's node 2 joins twice, a window each
        self.assertEqual(self.aligned("join", "lag_p50", "node", nth=0, before=0, after=1),
                         {("c", 0): (1, 300.0, 300.0, 300.0)})

    def test_aligned_pools_the_windows_rows(self):
        def pooled(field, **kw):
            got = self.run_cmd(simdb.cmd_aligned, event="first_read", field=field, per="run", nth=0, before=0,
                               after=1, node=None, pool=True, **kw)
            return [(r["cell"], r["rel_ms"], r["n"], r["p10"], r["p50"], r["p90"]) for r in got]

        # the five windows' 70 reads in one pool: 20 of lag 100 (kp1's), 20
        # of 150 (ks1's), 10 each of 200, 250 and 300
        self.assertEqual(pooled("reads"), [("c", 0, 70, 70, 70, 70)])
        self.assertEqual(pooled("lag_p50"), [("c", 0, 70, 150.0, 150.0, 150.0)])
        # in half-second bins, half of each window's reads a bin
        self.assertEqual(pooled("reads", bin=500), [("c", 0, 35, 35, 35, 35), ("c", 500, 35, 35, 35, 35)])

    def test_aligned_takes_the_forecast_logs_fields(self):
        # kp1's node 1 forecasts in the second after its first read: two of
        # its samples count, their errors 50 and -120
        self.assertEqual(self.aligned("first_read", "forecast_n", "run", before=0, after=1),
                         {("c", 0): (1, 2.0, 2.0, 2.0)})
        # of the two, the p50 is the first, -120
        self.assertEqual(self.aligned("first_read", "forecast_err_p50", "run", before=0, after=1),
                         {("c", 0): (1, -120.0, -120.0, -120.0)})
        self.assertEqual(self.aligned("first_read", "forecast_abs_err_p90", "run", before=0, after=1)[("c", 0)][2],
                         qc([50, 120], 90))
        # per node, node 2's sample after its own first read counts too
        got = self.aligned("first_read", "forecast_n", "node", before=0, after=1)
        self.assertEqual({k: (n, p50) for k, (n, _, p50, _) in got.items()}, {("c", 0): (2, 1.0)})
        self.assertIn("no forecast", self.refused("first_read", "forecast_n", "run", src=[S]))

    def test_json_gives_numbers_as_numbers(self):
        # DuckDB's CLI prints HUGEINT (sum() of integers), UBIGINT and
        # DECIMAL values as strings; a text column stays text
        got = self.run_cmd(simdb.cmd_sql, query="SELECT sum(seq) AS s, avg(seq)::DECIMAL(10, 2) AS d, "
                           "count(*)::UBIGINT AS u, NULL::HUGEINT AS n, '45' AS t, 'a;b' AS semi, [sum(seq)] AS l "
                           "FROM reads WHERE k = 'kd' -- the reads; all of them")
        self.assertEqual(got, [{"s": 1225, "d": 24.5, "u": 50, "n": None, "t": "45", "semi": "a;b", "l": [1225]}])
        self.assertIsInstance(got[0]["s"], int)
        # the last statement's rows, and a name the result repeats
        got = self.run_cmd(simdb.cmd_sql, query="CREATE TEMP TABLE x AS SELECT sum(seq) AS s FROM reads "
                           "WHERE k = 'kd'; SELECT s AS a, s * 2 AS a, 'x' AS b FROM x;")
        self.assertEqual(got, [{"a": 2450, "b": "x"}])
        # what DESCRIBE and SUMMARIZE give, and no rows
        summary = self.run_cmd(simdb.cmd_sql, query="SUMMARIZE SELECT seq FROM reads WHERE k = 'kd'")
        self.assertEqual((summary[0]["count"], summary[0]["null_percentage"]), (50, 0.0))
        self.assertEqual(self.run_cmd(simdb.cmd_sql, query="SELECT sum(seq) AS s FROM reads WHERE false "
                                      "GROUP BY k"), [])
        self.assertIn({"column_name": "seq", "column_type": "INTEGER", "null": "YES", "key": None, "default": None,
                       "extra": None}, self.run_cmd(simdb.cmd_sql, query="DESCRIBE reads"))
        # a statement that isn't a query runs as it is
        self.assertEqual(simdb.duck("SET threads = 1;"), [])
        # the commands' own queries
        got = self.run_cmd(simdb.cmd_pooled, by="none")
        self.assertTrue(all(isinstance(r["reads"], int) for r in got), got)

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
        for p in (*checkout(root),
                  mock.patch.object(query, "derive", lambda srcs: None),
                  mock.patch.object(store, "aws", self.aws),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
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
            # The filters, the last that matches a file's path below the
            # prefix deciding, as the CLI's do.
            filters = [(a, args[i + 1]) for i, a in enumerate(args) if a in ("--exclude", "--include")]

            def taken(rel):
                return ([a == "--include" for a, pat in filters if fnmatch.fnmatchcase(rel, pat)] or [True])[-1]

            # As the CLI does, for a prefix holding nothing too.
            dest.mkdir(parents=True, exist_ok=True)
            for f in sorted(prefix.rglob("*")):
                to = dest / f.relative_to(prefix)
                if f.is_file() and not to.exists() and taken(f.relative_to(prefix).as_posix()):
                    to.parent.mkdir(parents=True, exist_ok=True)
                    shutil.copyfile(f, to)
                    # As the CLI does: relative to its working directory.
                    out.append(f"download: s3://bkt/{f.relative_to(self.s3)} to {os.path.relpath(to)}\n")
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
        v, _ = views(self.args(cell))
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

    def test_a_whole_runs_reads_are_what_its_dry_run_lists(self):
        # The cache holding none of the run, the reads pull its tables
        # first (sim-data, every flush), then GETs each flush's samples.
        out, _ = self.cli("pull", "--run", RUN, "--reads")
        self.assertEqual([a[:3] for a in self.calls if a[1] == "sync"], [["s3", "sync", f"s3://bkt/sim-data/{RUN}/"]])
        self.assertEqual(self.gets(), {self.key(i, "reads") for i in ("i-1", "i-2", "i-3")}
                         | {self.key("i-2", "forecast")})
        mb = (sum(self.size(i, "reads") for i in ("i-1", "i-2", "i-3")) + self.size("i-2", "forecast")) / 1e6
        quote = (f"4 files, {mb:.1f} MB, for 7 jobs' reads and forecast; S3 holds none for 1 jobs (expired, or never "
                 "landed)")
        self.assertEqual(out, f"{RUN}: 4 sources in {simdb.cache_root() / 'cache' / 'farm' / RUN}; fetched {quote}\n")
        # Its dry run listed what it fetched: the cache's flushes' samples.
        for i, t in (("i-1", "reads"), ("i-2", "reads"), ("i-2", "forecast"), ("i-3", "reads")):
            (simdb.cache_root() / "cache" / src_of(i) / f"{t}.parquet").unlink()
        out, _ = self.cli("pull", "--run", RUN, "--reads", "--dry-run")
        self.assertEqual(out.splitlines()[0], f"{RUN}: would GET {quote}")
        self.assertEqual(self.calls, [])
        self.cli("pull", "--run", RUN, "--reads")
        # no sync: the cache holds the run's tables
        self.assertEqual([a[1] for a in self.calls], ["get-object"] * 4)
        cache = simdb.cache_root() / "cache"
        self.assertEqual({p.relative_to(cache).as_posix() for p in cache.glob("farm/**/*.parquet")
                          if p.stem in simdb.SAMPLE_TABLES},
                         {f"{src_of(i)}/{t}.parquet" for i, t in (("i-1", "reads"), ("i-2", "reads"),
                                                                  ("i-2", "forecast"), ("i-3", "reads"))})
        self.assertEqual(self.ledger()[-1]["tier2"], {"pull_get": 4})
        self.assertEqual(self.ledger()[-1]["down"],
                         {"internet": sum(self.size(i, "reads") for i in ("i-1", "i-2", "i-3"))
                          + self.size("i-2", "forecast")})

    def refused(self, *argv):
        """The message `pull ARGV` fails with (a SystemExit's, exit 1)."""
        with self.assertRaises(SystemExit) as cm:
            self.cli("pull", *argv)
        self.assertIsInstance(cm.exception.code, str)
        return cm.exception.code

    def test_a_matrix_or_a_suite_id_pulls_the_run_including_it(self):
        dest = simdb.cache_root() / "cache" / "farm" / RUN
        out, _ = self.cli("pull", "--run", RUN)
        self.assertEqual(out, f"{RUN}: 4 sources in {dest}\n")
        # A matrix's own state (target/sim-farm/<matrix>/) isn't a run's:
        # its name, and the suite farm run's id its matrices are named by,
        # name the one run including them, which the pull names.
        for name, what in ((C0, "a matrix"), ("sz0001", f"matrices {C0} to {C1} (2)")):
            self.calls.clear()
            out, _ = self.cli("pull", "--run", name)
            self.assertEqual(out, f"{name}: {what} of farm run {RUN}\n{RUN}: 4 sources in {dest}\n")
            self.assertEqual([a[2] for a in self.calls], [f"s3://bkt/sim-data/{RUN}/"])
        self.calls.clear()
        out, _ = self.cli("pull", "--run", C1, "--reads", "--dry-run")
        self.assertTrue(out.startswith(f"{C1}: a matrix of farm run {RUN}\n{RUN}: would GET 4 files, "), out)
        self.assertEqual(self.calls, [])
        # Without the run's state, the cache's flushes name it, its
        # matrices those its name joins.
        (simdb.main_checkout() / "target" / "sim-farm" / RUN / "matrices").unlink()
        self.assertEqual(store.farm_runs(), {RUN: [C0]})
        self.assertEqual(store.farm_run("sz0001"), store.Named(RUN, "sz0001", [C0]))

    def test_a_matrix_run_alone_that_s3_dropped_gives_way_to_the_run_including_it(self):
        # C0 also ran alone a month ago (a run of its name: its state's
        # matrices list), its data gone from S3 since: --run C0 names RUN,
        # started a day ago, and the line names both.
        farm = simdb.main_checkout() / "target" / "sim-farm"
        (farm / C0 / "matrices").write_text(f"{C0}\n")
        old, new = time.time() - 30 * 86400, time.time() - 86400
        os.utime(farm / C0 / "matrices", (old, old))
        os.utime(farm / RUN / "matrices", (new, new))
        dest = simdb.cache_root() / "cache" / "farm" / RUN
        out, _ = self.cli("pull", "--run", C0)
        self.assertEqual(out, f"{C0}: a matrix of farm run {RUN}, the one run it names started in the 7 days S3 keeps "
                              f"a run's data; it also names farm run {C0} (started {store.when(old)})\n"
                              f"{RUN}: 4 sources in {dest}\n")
        self.assertEqual([a[2] for a in self.calls], [f"s3://bkt/sim-data/{RUN}/"])
        # Both in S3's days: the run of that name, the line naming RUN.
        os.utime(farm / C0 / "matrices", (new - 86400, new - 86400))
        self.assertEqual(store.farm_run(C0), store.Named(C0, C0, also=[RUN]))
        self.assertEqual(store.farm_run(C0).text(), f"{C0}: farm run {C0}; it also names farm run {RUN} (started "
                                                    f"{store.when(new)}), which includes it: pull --run one by its "
                                                    "name for it")
        # A suite id two runs include is refused, as before.
        self.assertTrue(self.refused("--run", "sz0001").startswith("sz0001: 2 farm runs include it"))

    def test_a_name_several_runs_include_is_refused(self):
        other = f"sy0001-00+{C1}"
        (simdb.main_checkout() / "target" / "sim-farm" / other).mkdir()
        (simdb.main_checkout() / "target" / "sim-farm" / other / "matrices").write_text(f"sy0001-00\n{C1}\n")
        self.assertEqual(self.refused("--run", "sz0001"), f"sz0001: 2 farm runs include it; pull --run one of them:\n"
                                                          f"  {other}: matrix {C1}\n"
                                                          f"  {RUN}: matrices {C0} to {C1} (2)")
        self.assertEqual(self.refused("--run", C1, "--reads", "--cell", C1).splitlines()[0],
                         f"{C1}: 2 farm runs include it; pull --run one of them:")
        self.assertEqual(self.calls, [])
        out, _ = self.cli("pull", "--run", C0)
        self.assertTrue(out.startswith(f"{C0}: a matrix of farm run {RUN}\n"), out)

    def test_a_name_of_no_run_fails(self):
        # A suite farm run's id no run includes, and a name of nothing: S3
        # holds nothing of either, and the pull fails, saying where it
        # looked and the names nearest it, leaving the cache as it was.
        state = simdb.main_checkout() / "target" / "sim-farm"
        for name, near in (("sz0002", f"; the nearest: sz0001 (in {RUN}), {C1} (in {RUN}), {C0} (in {RUN})"),
                           ("nothing", "")):
            self.calls.clear()
            looked = (f"neither {state} nor the cache knows a run or matrix of that name, or matrices {name}-NN of "
                      f"a suite farm run{near}")
            self.assertEqual(self.refused("--run", name), f"{name}: no farm run is or includes it: S3 holds nothing "
                                                          f"under sim-data/{name}/, and {looked}")
            self.assertEqual([a[2] for a in self.calls], [f"s3://bkt/sim-data/{name}/"])
            self.assertFalse((simdb.cache_root() / "cache" / "farm" / name).exists())
            self.assertEqual(self.refused("--run", name, "--reads", "--cell", C0).splitlines(),
                             [f"{name}: no farm run is or includes it: S3 holds nothing under sim-data/{name}/, and "
                              f"{looked}"])
            # A dry run asks S3 nothing.
            self.calls.clear()
            self.assertEqual(self.refused("--run", name, "--reads", "--dry-run"),
                             f"{name}: no farm run is or includes it: {looked}")
            self.assertEqual(self.calls, [])
        # A run S3 no longer holds, the cache none of it.
        shutil.rmtree(self.s3 / "sim-data" / RUN)
        self.assertEqual(self.refused("--run", "sz0001"),
                         f"sz0001: matrices {C0} to {C1} (2) of farm run {RUN}; S3 holds nothing under "
                         f"sim-data/{RUN}/ (expired, or never landed), and the cache none of its flushes")
        self.assertEqual(self.refused("--run", RUN),
                         f"{RUN}: S3 holds nothing under sim-data/{RUN}/ (expired, or never landed), and the cache "
                         "none of its flushes")


# Two runs of one name, a testbed run of matrices tb-x and tb-y and its
# rerun at another commit, each a job a matrix and repeat (key, matrix,
# seed) in its flushes: the rerun's jobs key into the same cells and seeds.
# An instance the rerun adopted (i-k) holds a flush of each.
RUN_T = "tb-x+tb-y"
OLD, NEW = "abcd1" + "0" * 35, "abcd2" + "1" * 35
FIRST = (OLD, "2026-11-16T10:00:00+00:00", {"i-o1/0": [("o1", "tb-x", 1), ("o2", "tb-y", 1)],
                                            "i-k/0": [("o3", "tb-x", 2), ("o4", "tb-y", 2)]})
RERUN = (NEW, "2026-11-18T10:00:00+00:00", {"i-n1/0": [("n1", "tb-x", 1), ("n2", "tb-y", 1)],
                                            "i-k/1": [("n3", "tb-x", 2), ("n4", "tb-y", 2)]})
# A third run, at the rerun's commit, of tb-x alone (a setting of its
# changed, in matrices git doesn't hold): new jobs of tb-x's cells and
# seeds, tb-y's lines kept.
THIRD = (NEW, "2026-11-18T12:00:00+00:00", {"i-m1/0": [("m1", "tb-x", 1), ("m2", "tb-x", 2)]})


class OneRun(unittest.TestCase):
    """`pull --commit/--since` and `prune` over a scratch S3 holding two
    runs of one name, the aws CLI stubbed as SamplesPull's."""
    aws, cli, ledger, refused = SamplesPull.aws, SamplesPull.cli, SamplesPull.ledger, SamplesPull.refused

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        self.s3, self.calls = root / "s3", []
        for p in (*checkout(root),
                  mock.patch.object(query, "derive", lambda srcs: None),
                  mock.patch.object(store, "aws", self.aws),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        (root / "hack" / "sim-farm").mkdir(parents=True)
        (root / "hack" / "sim-farm" / "config.toml").write_text('s3_bucket = "bkt"\ns3_region = "us-west-1"\n')
        self.farm = root / "target" / "sim-farm"
        (self.farm / RUN_T).mkdir(parents=True)
        (self.farm / RUN_T / "matrices").write_text("tb-x\ntb-y\n")
        self.dest = simdb.cache_root() / "cache" / "farm" / RUN_T
        self.run_of(*FIRST)
        self.run_of(*RERUN)

    def run_of(self, commit, started, flushes):
        """A run of RUN_T at `commit`, its jobs started at `started`: its
        `flushes` in S3 (a runs and a reads table each, and the manifest)
        and a line a job in its matrix's rows.jsonl."""
        q = simdb.q
        # written once the flush landed
        at = (datetime.datetime.fromisoformat(started) + datetime.timedelta(minutes=2)).isoformat()
        for flush, jobs in flushes.items():
            src = f"farm/{RUN_T}/{flush}"
            data, samples = self.s3 / "sim-data" / RUN_T / flush, self.s3 / "sim-samples" / RUN_T / flush
            data.mkdir(parents=True)
            samples.mkdir(parents=True)
            runs = ", ".join(f"({q(k)}, {q(src)}, {q(m)}, 'main', {seed}, '{{}}')" for k, m, seed in jobs)
            reads = ", ".join(f"({q(k)}, {q(src)}, 0, {t}, {t // 100}, 10)" for k, _, _ in jobs for t in (0, 100))
            tables = {"runs": (data, f"SELECT * FROM (VALUES {runs}) t(k, src, cell, arm, seed, env)", len(jobs)),
                      "reads": (samples, f"SELECT * FROM (VALUES {reads}) t(k, src, viewer, t_ms, seq, lag_ms)",
                                2 * len(jobs))}
            manifest = {"keys": sorted(k for k, _, _ in jobs), "tables": {}}
            for t, (d, sql, n) in tables.items():
                simdb.duck(f"COPY ({sql}) TO {q(d / f'{t}.parquet')} (FORMAT parquet);", out=None)
                manifest["tables"][t] = {"rows": n, "bytes": (d / f"{t}.parquet").stat().st_size,
                                         "sha256": simdb.sha256(d / f"{t}.parquet")}
            (data / "manifest.json").write_text(json.dumps(manifest))
            for k, m, seed in jobs:
                (self.farm / m).mkdir(exist_ok=True)
                with open(self.farm / m / "rows.jsonl", "a") as f:
                    f.write(json.dumps({"id": k, "kind": "testbed", "arm": "main", "commit": commit, "repeat": seed,
                                        "host": flush.split("/")[0], "started": started, "at": at, "key": k,
                                        "src": src, "data": f"s3://bkt/sim-data/{RUN_T}/{flush}/", "reads": True,
                                        "row": {}}) + "\n")

    def held(self):
        """The cache's flushes of RUN_T, as instance/seq."""
        return {s.split("/", 2)[2] for s in store.sources([f"farm/{RUN_T}"])}

    def gets(self):
        """The objects the stub fetched one at a time."""
        return {a[a.index("--key") + 1] for a in self.calls if a[:2] == ["s3api", "get-object"]}

    def syncs(self):
        """The syncs the stub ran, each its source and the seqs its
        filters take (None: all), and forgets every call."""
        out = set()
        for a in self.calls:
            if a[:2] == ["s3", "sync"]:
                seqs = tuple(a[i + 1].removesuffix("/*") for i, x in enumerate(a) if x == "--include")
                out.add((a[2].removeprefix("s3://bkt/"), seqs if "--exclude" in a else None))
        self.calls.clear()
        return out

    def cells(self):
        """{(cell, seed): keys} of the views over the cache's flushes of
        RUN_T: a run of the name a key each."""
        v, _ = views(argparse.Namespace(src=[f"farm/{RUN_T}"], cell=None, arm=None, seed=None, bin=1000, since=None,
                                        until=None))
        got = {}
        for r in simdb.duck(v + "\nSELECT cell, seed, k FROM runs ORDER BY k;"):
            got.setdefault((r["cell"], r["seed"]), []).append(r["k"])
        return got

    def test_a_pull_by_commit_takes_one_run_of_the_name(self):
        out, _ = self.cli("pull", "--run", RUN_T, "--commit", "abcd2")
        self.assertEqual(out, f"{RUN_T}: 2 of the 4 flushes its matrices' lines name, those of its jobs of commit "
                              f"{NEW[:12]}\n{RUN_T}: 2 sources in {self.dest}\n")
        # A sync an instance, of its seqs the run's: the adopted i-k's 1
        # alone.
        self.assertEqual(self.syncs(), {(f"sim-data/{RUN_T}/i-n1/", ("0",)), (f"sim-data/{RUN_T}/i-k/", ("1",))})
        self.assertEqual(self.held(), {"i-n1/0", "i-k/1"})
        self.assertEqual(self.cells(), {("tb-x", 1): ["n1"], ("tb-y", 1): ["n2"], ("tb-x", 2): ["n3"],
                                        ("tb-y", 2): ["n4"]})
        # a LIST an instance's prefix, a GET a file (each flush's runs and
        # manifest)
        line = self.ledger()[-1]
        self.assertEqual((line["run"], line["tier1"], line["tier2"]), (RUN_T, {"pull_list": 2}, {"pull_get": 4}))
        self.assertEqual(line["down"]["internet"], sum(f.stat().st_size for f in self.dest.rglob("*") if f.is_file()))
        # Pulled again, nothing new comes; the other run's pull says the
        # cache holds this one's, which prune removes.
        self.cli("pull", "--run", RUN_T, "--commit", "abcd2")
        self.assertEqual(self.ledger()[-1]["tier2"], {"pull_get": 0})
        self.calls.clear()
        out, _ = self.cli("pull", "--run", RUN_T, "--commit", OLD)
        self.assertEqual(out.splitlines()[1:], [
            f"{RUN_T}: 2 sources in {self.dest}",
            f"{RUN_T}: the cache also holds 2 flushes of the run that those lines don't name (an earlier run's of its "
            f"name, say): simdb.py prune --run {RUN_T} --commit {OLD[:12]} removes them"])
        self.assertEqual(self.syncs(), {(f"sim-data/{RUN_T}/i-o1/", ("0",)), (f"sim-data/{RUN_T}/i-k/", ("0",))})

    def test_an_unfiltered_pull_takes_every_run_of_the_name(self):
        # As before: the whole prefix in one sync, both runs' jobs keyed
        # into the same cells and seeds.
        out, _ = self.cli("pull", "--run", RUN_T)
        self.assertEqual(out, f"{RUN_T}: 4 sources in {self.dest}\n")
        self.assertEqual(self.syncs(), {(f"sim-data/{RUN_T}/", None)})
        self.assertEqual(self.held(), {"i-o1/0", "i-k/0", "i-n1/0", "i-k/1"})
        self.assertEqual(self.cells(), {("tb-x", 1): ["n1", "o1"], ("tb-y", 1): ["n2", "o2"],
                                        ("tb-x", 2): ["n3", "o3"], ("tb-y", 2): ["n4", "o4"]})
        self.assertEqual(self.ledger()[-1]["tier1"], {"pull_list": 1})

    def test_prune_keeps_one_run_of_the_name(self):
        self.cli("pull", "--run", RUN_T)
        self.calls.clear()
        derived = simdb.cache_root() / "derived" / "verdicts" / "v=1"
        derived.mkdir(parents=True)
        for flush in ("i-o1/0", "i-n1/0"):
            (derived / f"farm~{RUN_T}~{flush.replace('/', '~')}.parquet").write_text("")
        out, _ = self.cli("prune", "--run", RUN_T, "--commit", "abcd2", "--dry-run")
        self.assertEqual(out, f"{RUN_T}: 2 of the 4 flushes its matrices' lines name, those of its jobs of commit "
                              f"{NEW[:12]}\n{RUN_T}: kept 2 of its flushes in the cache, would remove 2\n"
                              f"  farm/{RUN_T}/i-k/0\n  farm/{RUN_T}/i-o1/0\n")
        self.assertEqual(len(self.held()), 4)
        out, _ = self.cli("prune", "--run", RUN_T, "--commit", "abcd2")
        self.assertEqual(out.splitlines()[1], f"{RUN_T}: kept 2 of its flushes in the cache, removed 2: simdb.py "
                                              "index drops them from the catalog")
        self.assertEqual(self.held(), {"i-n1/0", "i-k/1"})
        self.assertEqual({p.name for p in derived.iterdir()}, {f"farm~{RUN_T}~i-n1~0.parquet"})
        # the instance it emptied goes; the adopted one stays
        self.assertEqual({p.name for p in self.dest.iterdir()}, {"i-n1", "i-k"})
        self.assertEqual(self.cells(), {("tb-x", 1): ["n1"], ("tb-y", 1): ["n2"], ("tb-x", 2): ["n3"],
                                        ("tb-y", 2): ["n4"]})
        self.assertEqual(self.calls, [])
        # A run the cache lacks part of: the pull that copies it.
        shutil.rmtree(self.dest / "i-n1")
        out, _ = self.cli("prune", "--run", RUN_T, "--commit", "abcd2")
        self.assertEqual(out.splitlines()[1:], [
            f"{RUN_T}: kept 1 of its flushes in the cache, removed 0",
            f"{RUN_T}: the cache lacks 1 of the flushes it keeps: simdb.py pull --run {RUN_T} --commit {NEW[:12]} "
            "copies them"])

    def test_the_time_parts_two_runs_of_one_commit(self):
        self.run_of(*THIRD)
        self.cli("pull", "--run", RUN_T, "--commit", "abcd2")
        self.assertEqual(self.held(), {"i-n1/0", "i-k/1", "i-m1/0"})
        self.assertEqual(self.cells()[("tb-x", 1)], ["m1", "n1"])
        out, _ = self.cli("prune", "--run", RUN_T, "--commit", "abcd2", "--since", "2026-11-18T11:00")
        self.assertEqual(out.splitlines()[0], f"{RUN_T}: 1 of the 5 flushes its matrices' lines name, those of its "
                                              f"jobs of commit {NEW[:12]} that ran since 2026-11-18 11:00:00 UTC")
        self.assertEqual(self.held(), {"i-m1/0"})
        # The time alone, in another zone: the same instant.
        shutil.rmtree(self.dest)
        self.calls.clear()
        self.cli("pull", "--run", RUN_T, "--since", "2026-11-18T12:00:00+01:00")
        self.assertEqual(self.syncs(), {(f"sim-data/{RUN_T}/i-m1/", ("0",))})
        self.assertEqual(self.cells(), {("tb-x", 1): ["m1"], ("tb-x", 2): ["m2"]})
        # The prune a pull names gives the time as the lines compare it.
        self.cli("pull", "--run", RUN_T, "--commit", "abcd2")
        out, _ = self.cli("pull", "--run", RUN_T, "--commit", "abcd2", "--since", "2026-11-18T11:00Z")
        self.assertEqual(out.splitlines()[-1], f"{RUN_T}: the cache also holds 2 flushes of the run that those lines "
                                               f"don't name (an earlier run's of its name, say): simdb.py prune --run "
                                               f"{RUN_T} --commit {NEW[:12]} --since 2026-11-18T11:00:00Z removes them")

    def test_reads_and_dry_runs_take_the_runs_flushes(self):
        # A dry run asks S3 nothing, and names the pull the cache lacks.
        self.assertEqual(self.refused("--run", RUN_T, "--commit", "abcd2", "--reads", "--dry-run"),
                         f"the cache lacks those flushes of farm/{RUN_T}: pull --run {RUN_T} --commit {NEW[:12]} "
                         "copies the tables that name its jobs")
        self.assertEqual(self.calls, [])
        self.cli("pull", "--run", RUN_T)
        out, _ = self.cli("pull", "--run", RUN_T, "--commit", "abcd2", "--reads", "--dry-run")
        self.assertEqual(out.splitlines()[1], f"{RUN_T}: would GET 2 files, 0.0 MB, for 4 jobs' reads")
        self.assertEqual({x.split()[0] for x in out.splitlines()[2:4]},
                         {f"s3://bkt/sim-samples/{RUN_T}/{f}/reads.parquet" for f in ("i-n1/0", "i-k/1")})
        # A cell's reads: of the run's flushes alone.
        self.calls.clear()
        self.cli("pull", "--run", RUN_T, "--commit", "abcd2", "--reads", "--cell", "tb-x")
        got = {a[a.index("--key") + 1] for a in self.calls if a[:2] == ["s3api", "get-object"]}
        self.assertEqual(got, {f"sim-samples/{RUN_T}/{f}/reads.parquet" for f in ("i-n1/0", "i-k/1")})
        # The whole run's reads: its tables first (a sync an instance, of
        # sim-data), then a GET a flush's samples.
        shutil.rmtree(self.dest)
        self.calls.clear()
        self.cli("pull", "--run", RUN_T, "--commit", "abcd2", "--reads")
        self.assertEqual(self.gets(), {f"sim-samples/{RUN_T}/{f}/reads.parquet" for f in ("i-n1/0", "i-k/1")})
        self.assertEqual(self.syncs(), {(f"sim-data/{RUN_T}/i-n1/", ("0",)), (f"sim-data/{RUN_T}/i-k/", ("1",))})
        self.assertEqual({p.relative_to(self.dest).as_posix() for p in self.dest.rglob("reads.parquet")},
                         {"i-n1/0/reads.parquet", "i-k/1/reads.parquet"})
        # A dry run of those flushes, the cache lacking one: what it would
        # GET of the other, and the one its pull copies first.
        shutil.rmtree(self.dest / "i-k")
        (self.dest / "i-n1" / "0" / "reads.parquet").unlink()
        out, _ = self.cli("pull", "--run", RUN_T, "--commit", "abcd2", "--reads", "--dry-run")
        self.assertEqual(out.splitlines()[1:], [
            f"{RUN_T}: would GET 1 files, 0.0 MB, for 2 jobs' reads",
            f"  s3://bkt/sim-samples/{RUN_T}/i-n1/0/reads.parquet  0.0 MB  2 jobs",
            f"{RUN_T}: the cache lacks 1 of those flushes, whose samples a pull GETs once it has copied their tables"])
        self.assertEqual(self.calls, [])

    def test_after_a_prune_the_reads_are_the_kept_flushes(self):
        # Every run of the name pulled, the other run's flushes pruned: the
        # reads, and their dry run, are the kept flushes' alone, and the
        # pruned ones stay out.
        self.cli("pull", "--run", RUN_T)
        self.cli("prune", "--run", RUN_T, "--commit", "abcd2")
        self.calls.clear()
        out, _ = self.cli("pull", "--run", RUN_T, "--reads", "--dry-run")
        self.assertEqual({x.split()[0] for x in out.splitlines()[1:]},
                         {f"s3://bkt/sim-samples/{RUN_T}/{f}/reads.parquet" for f in ("i-n1/0", "i-k/1")})
        self.assertTrue(out.startswith(f"{RUN_T}: would GET 2 files, 0.0 MB, for 4 jobs' reads\n"), out)
        out, _ = self.cli("pull", "--run", RUN_T, "--reads")
        self.assertEqual(out, f"{RUN_T}: 2 sources in {self.dest}; fetched 2 files, 0.0 MB, for 4 jobs' reads\n")
        self.assertEqual(self.gets(), {f"sim-samples/{RUN_T}/{f}/reads.parquet" for f in ("i-n1/0", "i-k/1")})
        self.assertEqual(self.syncs(), set())
        self.assertEqual(self.held(), {"i-n1/0", "i-k/1"})
        line = self.ledger()[-1]
        self.assertEqual((line["tier1"], line["tier2"]), ({}, {"pull_get": 2}))
        self.assertEqual(line["down"]["internet"], sum(f.stat().st_size for f in self.dest.rglob("reads.parquet")))
        # Pulled again: nothing to fetch, and no ledger line.
        n = len(self.ledger())
        out, _ = self.cli("pull", "--run", RUN_T, "--reads")
        self.assertEqual(out, f"{RUN_T}: 2 sources in {self.dest}; the cache holds what S3 has of their samples\n")
        self.assertEqual((self.calls, len(self.ledger())), ([], n))
        # The tables of every flush under the name come back only with a
        # pull without --reads.
        self.cli("pull", "--run", RUN_T)
        self.assertEqual(self.held(), {"i-o1/0", "i-k/0", "i-n1/0", "i-k/1"})

    def test_a_lines_time_is_its_jobs_start_else_when_it_was_written(self):
        # A testbed line's started, a test line's at (it has no started),
        # UTC where it names no zone.
        at = datetime.datetime(2026, 11, 18, 10, 2, tzinfo=datetime.UTC)
        self.assertEqual(store.line_time({"started": "2026-11-18T10:00:00+00:00", "at": at.isoformat()}),
                         at - datetime.timedelta(minutes=2))
        self.assertEqual(store.line_time({"at": "2026-11-18T10:02:00"}), at)
        self.assertEqual(store.line_time({"started": None, "at": "2026-11-18T11:02:00+01:00"}), at)
        self.assertIsNone(store.line_time({}))
        # A test matrix's index.jsonl names flushes as a testbed matrix's
        # rows.jsonl does.
        line = {"key": "t1", "commit": NEW, "at": "2026-11-18T13:00:00+00:00", "src": f"farm/{RUN_T}/i-t/0"}
        (self.farm / "tb-x" / "index.jsonl").write_text(json.dumps(line) + "\n")
        self.assertEqual(store.run_flushes(RUN_T, ["abcd2"], store.instant("2026-11-18T12:30")).srcs,
                         [f"farm/{RUN_T}/i-t/0"])

    def test_what_names_no_run_is_refused(self):
        self.assertEqual(self.refused("--run", RUN_T, "--commit", "abcd"),
                         f"--commit abcd: 2 of the commits {RUN_T}'s lines record start with it: {OLD[:12]}, "
                         f"{NEW[:12]}")
        self.assertEqual(self.refused("--run", RUN_T, "--commit", "abc"),
                         "--commit abc: not a commit's sha or a prefix of one, 4 to 40 lowercase hex digits")
        listed = (f"  {NEW[:12]}: 2 flushes, their jobs ran at 2026-11-18 10:00:00 UTC\n"
                  f"  {OLD[:12]}: 2 flushes, their jobs ran at 2026-11-16 10:00:00 UTC")
        self.assertEqual(self.refused("--run", RUN_T, "--commit", "cafe"),
                         f"{RUN_T}: none of its matrices' lines is of commit cafe; the commits they record:\n{listed}")
        self.assertEqual(self.refused("--run", RUN_T, "--commit", "abcd2", "--since", "2026-11-19"),
                         f"{RUN_T}: none of its matrices' lines is of a job of commit {NEW[:12]} that ran since "
                         f"2026-11-19 00:00:00 UTC; the commits they record:\n{listed}")
        self.assertEqual(self.refused("--run", RUN_T, "--commit", "AB34"),
                         "--commit AB34: not a commit's sha or a prefix of one, 4 to 40 lowercase hex digits")
        self.assertEqual(self.refused("--run", RUN_T, "--since", "yesterday"),
                         "yesterday: not an ISO 8601 time (2026-11-18T16:32, UTC where it names no zone)")
        with self.assertRaises(SystemExit) as cm:
            self.cli("prune", "--run", RUN_T)
        self.assertEqual(cm.exception.code, "prune: give --commit or --since, the jobs whose flushes the cache keeps")
        # A name of no run: unknown's line.
        self.assertTrue(self.refused("--run", "tb-z", "--commit", "abcd2").startswith(
            "tb-z: no farm run is or includes it: neither "), self.refused("--run", "tb-z", "--commit", "abcd2"))
        # A run no line names a flush of (its matrices' state gone).
        for m in ("tb-x", "tb-y"):
            (self.farm / m / "rows.jsonl").unlink()
        self.assertTrue(self.refused("--run", RUN_T, "--commit", "abcd2").startswith(
            f"{RUN_T}: no line of its matrices' rows.jsonl or index.jsonl in {self.farm} names a flush of it"))
        self.assertEqual(self.calls, [])


def audience(secs, stays, sim="ppspp", tree_secs=3, stay_info=True):
    """A series of `secs` seconds whose viewers come and go by `stays`, each
    (arrive_s, depart_s): arrive_s None for one there from the start,
    depart_s None for one that stays. A viewer there reads its 10 chunks a
    second at 200 ms (300 ms in its first `tree_secs`, where its pushes
    don't come yet: its tree forming), as `sim` encodes a second it isn't
    there: the ppspp sim's -1 lag and 0 frames, strims-sim's -1 frames
    (and lag). With `stay_info`, the viewers table's arrive_ms and
    depart_ms (-1000 for the start, None for no departure)."""
    s = {"rate": 10 * 4096, "chunk": 4096, "frame": 4096, "secs": secs, "seed_up": [[0, 10 ** 9]],
         "viewer_up": 10 ** 9, "lag": [], "frames": [], "present": [0] * secs, "hits": [0] * secs}
    for a, d in stays:
        lag, frames = [], []
        for k in range(secs):
            there = (a is None or a <= k + 1) and (d is None or k < d)
            settled = there and (a is None and k >= tree_secs or a is not None and k >= a + tree_secs)
            # ppspp counts a viewer present at both of a second's edges,
            # strims-sim from the second it boots in.
            arrived = a is None or (a < k + 1 if sim == "strims-sim" else a <= k)
            if arrived and (d is None or d >= k + 1):
                s["present"][k] += 1
                s["hits"][k] += 10 if settled else 0
            if not there or a is not None and k < a:
                # Not there, or there only from part-way through its second.
                lag.append(-1)
                frames.append(-1 if sim == "strims-sim" and not there else 0)
                continue
            lag.append(200 if settled else 300)
            frames.append(10)
        s["lag"].append(lag)
        s["frames"].append(frames)
    if stay_info:
        s["arrive_ms"] = [-1000 if a is None else a * 1000 for a, _ in stays]
        s["depart_ms"] = [None if d is None else d * 1000 for _, d in stays]
    return s


class PerViewerVerdict(unittest.TestCase):
    """The verdict counts each viewer only while it is there and
    past its own startup (verdict.presence: its join's window, the phase
    rule's)."""

    a = verdict

    def verdict(self, s):
        v = self.a.verdict(s)
        return v, self.a.collapsed(v)

    def test_t0_reads_as_without_stays(self):
        # Viewers there from the start, as every t0 run's are: the stays
        # change nothing, and the run's startup is STARTUP_SECS.
        stays = [(None, None)] * 12
        for sim in ("ppspp", "strims-sim"):
            with_stays = self.a.verdict(audience(60, stays, sim))
            without = self.a.verdict(audience(60, stays, sim, stay_info=False))
            self.assertEqual(with_stays, without, sim)
            self.assertEqual(with_stays["startup"], self.a.STARTUP_SECS)
            # Its first seconds are judged as ever: trees forming.
            self.assertFalse(all(with_stays["healthy"]))

    def test_trickle_reads_healthy_throughout(self):
        # 20 viewers arrive a second apart from 0.5 s: none counts before
        # its join plus join's horizon (its own startup), so trees forming
        # around arrivals read as nothing, where counting them from their
        # arrival reads the first seconds' trees as failing. The run's
        # startup is the stream's start's, every viewer arriving later or
        # not.
        stays = [(k + 0.5, None) for k in range(20)]
        own = verdict.ANCHORS["join"]["horizon_s"]
        for sim in ("ppspp", "strims-sim"):
            v, collapsed = self.verdict(audience(60, stays, sim))
            self.assertTrue(all(v["healthy"]), sim)
            self.assertFalse(collapsed, sim)
            self.assertEqual(v["startup"], self.a.STARTUP_SECS, sim)
            first = [st.index("counts") for st in self.a.presence(audience(60, stays, sim))]
            self.assertEqual(first[:2], [own, own + 1], sim)
        v, _ = self.verdict(audience(60, stays, "ppspp", stay_info=False))
        self.assertFalse(all(v["healthy"]))
        # strims-sim's -1 frames carry the arrivals without the table, those
        # after the stream's first second (one within it reads as there from
        # the start).
        later = [(a + 1, d) for a, d in stays]
        self.assertEqual(self.a.verdict(audience(60, later, "strims-sim", stay_info=False))["healthy"],
                         [True] * 60)

    def test_a_burst_of_joins_into_a_warm_swarm_reads_healthy(self):
        # 5 viewers there from the start, 30 more within 5 s at 30 s: the
        # burst's trees forming would read as the warm swarm's trees
        # failing.
        stays = [(None, None)] * 5 + [(30 + k / 6, None) for k in range(30)]
        v, collapsed = self.verdict(audience(70, stays))
        self.assertTrue(all(v["healthy"][self.a.STARTUP_SECS:]))
        self.assertFalse(collapsed)
        old, _ = self.verdict(audience(70, stays, stay_info=False))
        self.assertFalse(all(old["healthy"][self.a.STARTUP_SECS:]))

    def test_a_mass_departure_is_no_collapse_by_construction(self):
        # 70 % of 20 viewers leave between 30 and 37 s and the rest keep
        # reading: the leavers' seconds after they left don't count, where
        # reading them as falling behind live collapses the run.
        stays = [(None, 30 + k / 2) for k in range(14)] + [(None, None)] * 6
        for sim in ("ppspp", "strims-sim"):
            v, collapsed = self.verdict(audience(60, stays, sim))
            self.assertTrue(all(v["healthy"][self.a.STARTUP_SECS:]), sim)
            self.assertFalse(collapsed, sim)
        _, collapsed = self.verdict(audience(60, stays, stay_info=False))
        self.assertTrue(collapsed)
        # strims-sim's -1 frames carry the departures without the table.
        _, collapsed = self.verdict(audience(60, stays, "strims-sim", stay_info=False))
        self.assertFalse(collapsed)
        # A stayer left behind still counts: one that stops reading at 40 s
        # falls behind live, and the swarm's lag p90 with it.
        s = audience(60, stays[:14] + [(None, None)] * 2)
        for k in range(40, 60):
            s["lag"][-1][k] = s["lag"][-2][k] = -1
            s["frames"][-1][k] = s["frames"][-2][k] = 0
        _, collapsed = self.verdict(s)
        self.assertTrue(collapsed)

    def test_viewer_phases_are_the_runs_windows_and_its_own_joins(self):
        # The seed's uplink steps down at 30 s; viewer 8 arrives at 20.5 s
        # and reads from 21 s, viewer 9 (hostile) leaves at 40.5 s. The
        # series has no events: they come from seed_up and the stays.
        s = audience(60, [(None, None), (20.5, None), (None, 40.5)])
        s["seed_up"] = [[0, 10 ** 9], [30, 5 * 10 ** 8]]
        s["hostile"], s["viewer"] = [2], [7, 8, 9]
        v = self.a.verdict(s)
        start = self.a.STARTUP_SECS
        # The run: its startup, steady, the step down's window, then the
        # departure's (at 40.5 s, holding second 40 from its midpoint).
        self.assertEqual(self.a.phases(s, v), ["startup"] * start + ["steady"] * (30 - start)
                         + ["after_down"] * 10 + ["after_departure"] * 20)
        # Viewer 8's own startup from its join's second, until the step
        # down, later, takes its phase; viewer 9's seconds end at its
        # departure's, which it was there part of.
        self.assertEqual(verdict.viewer_phases(s, v), [
            (7, "startup", 0, start), (7, "steady", start, 30), (7, "after_down", 30, 40),
            (7, "after_departure", 40, 60),
            (8, "startup", 20, 30), (8, "after_down", 30, 40), (8, "after_departure", 40, 60),
            (9, "startup", 0, start), (9, "steady", start, 30), (9, "after_down", 30, 40),
            (9, "after_departure", 40, 41)])
        # A second's phase is the run's but in an arrival's own startup,
        # which doesn't count in the verdict for join's horizon from its
        # join, the step down's phase or not.
        ph = self.a.phases(s, v)
        st = self.a.presence(s)
        for i, p, a, b in verdict.viewer_phases(s, v):
            self.assertTrue(all(p == ("startup" if i == 8 and k < 30 else ph[k]) for k in range(a, b)))
        own = verdict.ANCHORS["join"]["horizon_s"]
        self.assertEqual([k for k, x in enumerate(st[1]) if x == "own"], list(range(20, 20 + own)))
        self.assertEqual((st[2][39], st[2][40], st[2][41]), ("counts", "leaving", None))
        # Without the series' ids, a viewer is its place.
        del s["viewer"]
        self.assertEqual([x[0] for x in verdict.viewer_phases(s, v)], [0] * 4 + [1] * 3 + [2] * 4)

    def test_present_join_and_leave_count_every_viewer(self):
        # The events read the viewers present as the series counts them,
        # arrivals in their grace and all.
        stays = [(None, None)] * 4 + [(10.5, None), (11.5, 40)]
        s = audience(60, stays)
        got = {(t, kind, x) for t, kind, x in self.a.events(s, self.a.verdict(s)) if kind.startswith("present")}
        self.assertEqual(got, {(11, "present_join", 1), (12, "present_join", 1), (40, "present_leave", 1)})

    def test_series_of_lays_each_viewer_out_by_second(self):
        # strims-sim's viewer_seconds has no row for a second its viewer
        # wasn't there in; the viewers table carries the stays.
        secs = [{"k": "k", "src": "s", "sec": k, "present": 1} for k in range(4)]
        vs = [{"viewer": 0, "sec": [0, 1, 2, 3], "lag": [5, 5, 5, 5], "frames": [1, 1, 1, 1]},
              {"viewer": 1, "sec": [2, 3], "lag": [7, 7], "frames": [2, 2]}]
        stays = [{"viewer": 0, "arrive_ms": -3000, "depart_ms": None},
                 {"viewer": 1, "arrive_ms": 1500, "depart_ms": None}]
        s = simdb.series_of({"secs": 4}, secs, vs, stays)
        self.assertEqual(s["frames"], [[1, 1, 1, 1], [-1, -1, 2, 2]])
        self.assertEqual(s["lag"], [[5, 5, 5, 5], [-1, -1, 7, 7]])
        self.assertEqual((s["arrive_ms"], s["depart_ms"]), ([-3000, 1500], [None, None]))
        self.assertNotIn("arrive_ms", simdb.series_of({"secs": 4}, secs, vs))
        self.assertEqual(s["viewer"], [0, 1])


class Derived(unittest.TestCase):
    """The derived tables hold their declared columns alone, under a
    version that moves with them."""

    SRC = "suite/wt/d"
    # k: the viewers' stays (audience's): every one there from the start,
    # or each arriving later
    STAYS = {"kt0": [(None, None)] * 6, "ktr": [(k + 0.5, None) for k in range(6)]}

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        series = {(self.SRC, k): ({}, audience(40, stays)) for k, stays in self.STAYS.items()}
        for p in (*checkout(root),
                  mock.patch.object(verdict, "load_series", lambda pairs: {p: series[p] for p in pairs}),
                  mock.patch.object(query, "WORKERS", 0),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        d = simdb.cache_root() / "cache" / self.SRC
        d.mkdir(parents=True)
        vals = ", ".join(f"({simdb.q(k)}, {simdb.q(self.SRC)}, 'c', 'a', 1)" for k in self.STAYS)
        simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, cell, arm, seed)) "
                   f"TO {simdb.q(d / 'runs.parquet')} (FORMAT parquet);", out=None)

    def columns(self, path):
        return [r["column_name"] for r in simdb.duck(
            f"DESCRIBE SELECT * FROM read_parquet({simdb.q(path)}, hive_partitioning=false);")]

    def view(self, query):
        v, have = views(argparse.Namespace(src=None, cell=None, arm=None, seed=None))
        return simdb.duck(v + "\n" + query + ";"), have

    def test_a_table_holds_its_columns_alone(self):
        path = simdb.cache_root() / "derived" / "t" / "v=abc" / "x.parquet"
        simdb.write_table(path, simdb.DERIVED["episodes"], [["k", "s", "collapse", 1, 9, 8]])
        self.assertEqual(self.columns(path), list(simdb.DERIVED["episodes"]))

    def test_viewer_phases_hold_each_viewers_own_startup(self):
        got, have = self.view("SELECT k, viewer, phase, start, stop FROM viewer_phases ORDER BY ALL")
        self.assertIn("viewer_phases", have)
        a = verdict
        by = {}
        for r in got:
            by.setdefault((r["k"], r["viewer"]), []).append((r["phase"], r["start"], r["stop"]))
        # There from the start: the run's startup, then steady; arriving at
        # i + 0.5 s: its own startup, join's horizon from its join's second,
        # then the run's while that lasts.
        own = verdict.ANCHORS["join"]["horizon_s"]
        for i in range(6):
            self.assertEqual(by[("kt0", i)], [("startup", 0, a.STARTUP_SECS), ("steady", a.STARTUP_SECS, 40)])
            end = max(i + own, a.STARTUP_SECS)
            self.assertEqual(by[("ktr", i)], [("startup", i, end), ("steady", end, 40)])

    def test_verdicts_carry_the_startup(self):
        got, _ = self.view("SELECT k, startup_s, collapsed FROM verdicts ORDER BY k")
        a = verdict
        self.assertEqual({r["k"]: r["startup_s"] for r in got}, {"kt0": a.STARTUP_SECS, "ktr": a.STARTUP_SECS})
        name = self.SRC.replace("/", "~") + ".parquet"
        for t in [*simdb.DERIVED, "verdicts"]:
            cols = self.columns(simdb.derived_dir(t) / name)
            self.assertNotIn("v", cols, t)
            self.assertEqual(cols[:len(simdb.DERIVED.get(t, simdb.VERDICT_COLS))],
                             list(simdb.DERIVED.get(t, simdb.VERDICT_COLS)), t)

    def test_a_table_made_before_is_stale(self):
        # Verdicts made without startup_s, under VERDICT_VERSION alone: the
        # views make their own rather than read them, and the version moves
        # with the columns.
        a = verdict
        name = self.SRC.replace("/", "~") + ".parquet"
        old = {c: t for c, t in simdb.VERDICT_COLS.items() if c != "startup_s"}
        made = [[k, self.SRC] + [None] * (len(old) - 2) for k in self.STAYS]
        for t, cols, rows in [*((t, c, []) for t, c in simdb.DERIVED.items()), ("verdicts", old, made)]:
            simdb.write_table(simdb.cache_root() / "derived" / t / f"v={a.VERDICT_VERSION}" / name, cols, rows)
        got, _ = self.view("SELECT k, startup_s FROM verdicts ORDER BY k")
        self.assertEqual([(r["k"], r["startup_s"]) for r in got], [("kt0", a.STARTUP_SECS), ("ktr", a.STARTUP_SECS)])
        now = simdb.derived_version()
        self.addCleanup(simdb.derived_version.cache_clear)
        simdb.derived_version.cache_clear()
        with mock.patch.object(verdict, "VERDICT_COLS", old):
            self.assertNotEqual(simdb.derived_version(), now)

    def test_a_stray_v_column_is_left_out(self):
        # A file written beside its CSV in the v= directory, with DuckDB's
        # hive detection on, holds a `v` column; the views read it and a
        # file without alike.
        simdb.derive([self.SRC])
        d = simdb.derived_dir("episodes")
        stray = d / "suite~wt~e.parquet"
        simdb.duck(f"COPY (SELECT * FROM read_parquet({simdb.q(d / 'suite~wt~d.parquet')})) "
                   f"TO {simdb.q(stray)} (FORMAT parquet);", out=None)
        self.assertIn("v", self.columns(stray))
        f = simdb.derived_files("episodes", [self.SRC, "suite/wt/e"])
        got = simdb.duck(f"DESCRIBE SELECT x.* FROM {f};")
        self.assertEqual([r["column_name"] for r in got], list(simdb.DERIVED["episodes"]))


class RuleEvents(unittest.TestCase):
    """load_series gives the verdict the events its phase rule reads, by
    their current ids: a source of last week's, holding retired ids
    (strims-sim's up_step and uplink, the ppspp sim's uplink) and no
    starting levels, judges as one holding today's."""

    # (sim, its runs' events: (t_s, kind, node, value), and its viewers
    # table's node column)
    SOURCES = {
        "suite/wt/old": ("strims-sim", [(-5, "join", 1, 0), (-5, "join", 2, 0), (0.3, "first_frame", 1, 300),
                                        (30, "up_step", 0, 5 * 10 ** 8), (40, "uplink", 1, 10 ** 5),
                                        (45, "depart", 2, 1)], "host"),
        "suite/wt/new": ("strims-sim", [(-5, "join", 1, 0), (-5, "join", 2, 0), (-5, "uplink_rate", 0, 10 ** 9),
                                        (0.3, "first_read", 1, 300), (30, "uplink_rate", 0, 5 * 10 ** 8),
                                        (40, "rate_limit", 1, 10 ** 5), (45, "depart", 2, 1)], "node"),
        "suite/wt/pp-old": ("ppspp", [(-1, "join", 1, 0), (-1, "join", 2, 0), (30, "uplink", 0, 5 * 10 ** 8),
                                      (45, "depart", 2, 1)], "node"),
        "suite/wt/pp-new": ("ppspp", [(-1, "join", 1, 0), (-1, "join", 2, 0), (-1, "uplink_rate", 0, 10 ** 9),
                                      (30, "uplink_rate", 0, 5 * 10 ** 8), (45, "depart", 2, 1)], "node"),
    }
    SECS = 60

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (*checkout(root), mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        q = simdb.q
        meta = {"rate": 10 * 4096, "chunk": 4096, "frame": 4096, "secs": self.SECS, "viewer_up": 10 ** 9,
                "seed_up": [[0, 10 ** 9], [30, 5 * 10 ** 8]]}
        for src, (sim, events, node) in self.SOURCES.items():
            d = simdb.cache_root() / "cache" / src
            d.mkdir(parents=True)
            copy = lambda name, cols, rows: simdb.duck(
                f"COPY (SELECT * FROM (VALUES {', '.join(rows)}) t({cols})) TO {q(d / f'{name}.parquet')} "
                "(FORMAT parquet);", out=None)
            copy("runs", "k, src, sim, meta", [f"('k', {q(src)}, {q(sim)}, {q(json.dumps({'sim': sim, **meta}))})"])
            copy("seconds", "k, src, sec, present, hits", [f"('k', {q(src)}, {k}, 2, 20)" for k in range(self.SECS)])
            copy("viewer_seconds", "k, src, viewer, sec, lag_ms, frames",
                 [f"('k', {q(src)}, {v}, {k}, 200, 10)" for v in (0, 1) for k in range(self.SECS)])
            copy("viewers", f"k, src, viewer, {node}, arrive_ms, depart_ms",
                 [f"('k', {q(src)}, 0, 1, -5000, NULL)", f"('k', {q(src)}, 1, 2, -5000, 45000)"])
            copy("events", "k, src, t_us, kind, node, peer, value, detail",
                 [f"('k', {q(src)}, {int(t * 10 ** 6)}, {q(kind)}, {n}, -1, {x}, '')" for t, kind, n, x in events])

    def judged(self, src):
        (meta, s), = verdict.load_series([(src, "k")]).values()
        v = verdict.verdict(s)
        return s, {"phases": verdict.phases(s, v), "viewer_phases": verdict.viewer_phases(s, v),
                   "windows": verdict.phase_windows(s), "steps": verdict.steps(s)}

    def test_retired_ids_judge_as_current_ones(self):
        old, got = self.judged("suite/wt/old")
        new, want = self.judged("suite/wt/new")
        self.assertEqual(got, want)
        # The rule's kinds alone, by their current ids; the viewers' nodes
        # strims-sim's hosts.
        self.assertEqual([e[1] for e in old["events"]], ["join", "join", "uplink_rate", "rate_limit", "depart"])
        self.assertEqual(old["node"], [1, 2])
        self.assertEqual([(kind, p, a) for _, kind, _, p, a, *_ in want["windows"]],
                         [("start", "startup", 0), ("uplink_rate", "after_down", 30), ("rate_limit", "after_down", 40),
                          ("depart", "after_departure", 45)])
        # The ppspp sim's uplink is its uplink_rate (strims-sim's its
        # rate_limit).
        _, pp_old = self.judged("suite/wt/pp-old")
        _, pp_new = self.judged("suite/wt/pp-new")
        self.assertEqual(pp_old, pp_new)
        self.assertEqual([kind for _, kind, *_ in pp_new["windows"]], ["start", "uplink_rate", "depart"])

    def test_a_source_without_events_implies_them(self):
        # Its seed's uplink from seed_up and its viewers' stays: no cap.
        (simdb.cache_root() / "cache" / "suite/wt/new" / "events.parquet").unlink()
        s, got = self.judged("suite/wt/new")
        self.assertNotIn("events", s)
        self.assertEqual([(kind, node, p, a) for _, kind, node, p, a, *_ in got["windows"]],
                         [("start", None, "startup", 0), ("uplink_rate", 0, "after_down", 30),
                          ("depart", 2, "after_departure", 45)])
        self.assertEqual(got["phases"][29:], ["steady"] + ["after_down"] * 15 + ["after_departure"] * 15)


class Playback(unittest.TestCase):
    """macros.sql's playback(delay_ms) plays `reads` as the sims' `stalls`
    tables have them (strims-wan's playout): over runs made up as its unit
    tests' are, and over runs of both sims trimmed to a few viewers
    (testdata/playback/<sim>/, each a run's reads and the stalls it
    wrote)."""

    SRC = "suite/wt/pb"
    # k: viewer: its reads (t_ms, seq, lag_ms). kq's viewer 0 has a unit
    # 400 ms late at 1250; viewer 1 a unit read again much later, which
    # counts at its first read, and two due together, the later 250 ms
    # late; viewer 2 a late first unit, which starts playback late. kr's
    # second unit is late at both delays.
    READS = {"kq": {0: [(1200, 0, 1200), (1300, 1, 1200), (1850, 2, 1650), (1500, 3, 1200)],
                    1: [(100, 0, 100), (200, 1, 100), (9000, 1, 8900), (1600, 2, 1400), (1700, 3, 1500)],
                    2: [(2000, 0, 2000), (2100, 1, 2000)]},
             "kr": {0: [(1200, 0, 1200), (6000, 1, 5900)]}}
    FIXTURES = Path(__file__).with_name("testdata") / "playback"
    COLS = {"reads": {"viewer": "BIGINT", "t_ms": "BIGINT", "seq": "BIGINT", "lag_ms": "BIGINT"},
            "stalls": {"viewer": "BIGINT", "delay_ms": "BIGINT", "t_ms": "BIGINT", "stall_ms": "BIGINT"}}

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (*checkout(root),
                  mock.patch.object(query, "derive", lambda srcs: None),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        a = verdict
        self.delays = [a.DELIVERY_TARGET_MS, a.DELIVERY_MAX_MS]

    def runs(self, src, keys):
        """The scratch cache's dir of `src`, holding runs `keys`."""
        d = simdb.cache_root() / "cache" / src
        d.mkdir(parents=True, exist_ok=True)
        vals = ", ".join(f"({simdb.q(k)}, {simdb.q(src)}, 'c', 'a', 1)" for k in keys)
        simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, cell, arm, seed)) "
                   f"TO {simdb.q(d / 'runs.parquet')} (FORMAT parquet);", out=None)
        return d

    def rows(self, query):
        v, _ = views(argparse.Namespace(src=None, cell=None, arm=None, seed=None))
        return sorted((r["k"], r["viewer"], r["delay_ms"], r["t_ms"], r["stall_ms"])
                      for r in simdb.duck(v + "\n" + query + ";"))

    def played(self):
        return self.rows(" UNION ALL ".join(f"SELECT * FROM playback({d})" for d in self.delays))

    def test_a_stall_is_each_rise_of_the_offset_past_the_first_unit(self):
        d = self.runs(self.SRC, self.READS)
        vals = ", ".join(f"({simdb.q(k)}, {simdb.q(self.SRC)}, {v}, {t}, {seq}, {lag})"
                         for k, vs in self.READS.items() for v, rs in vs.items() for t, seq, lag in rs)
        simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, viewer, t_ms, seq, lag_ms)) "
                   f"TO {simdb.q(d / 'reads.parquet')} (FORMAT parquet);", out=None)
        self.assertEqual(self.delays, [1250, 4250])
        self.assertEqual(self.played(), [("kq", 0, 1250, 1450, 400), ("kq", 1, 1250, 1450, 250),
                                         ("kr", 0, 1250, 1350, 4650), ("kr", 0, 4250, 4350, 1650)])

    def test_playback_gives_the_sims_own_stalls(self):
        sims = sorted(p.name for p in self.FIXTURES.iterdir())
        self.assertEqual(sims, ["ppspp", "strims-sim"])
        for sim in sims:
            src = f"suite/wt/{sim}"
            d = self.runs(src, [sim])
            for t, cols in self.COLS.items():
                c = "{" + ", ".join(f"{simdb.q(n)}: {simdb.q(typ)}" for n, typ in cols.items()) + "}"
                f = self.FIXTURES / sim / f"{t}.csv.gz"
                simdb.duck(f"COPY (SELECT {simdb.q(sim)} AS k, {simdb.q(src)} AS src, * FROM read_csv({simdb.q(f)}, "
                           f"header=true, auto_detect=false, columns={c})) "
                           f"TO {simdb.q(d / f'{t}.parquet')} (FORMAT parquet);", out=None)
        stalls = self.rows("SELECT * FROM stalls")
        for sim in sims:
            for delay in self.delays:
                self.assertTrue(any(s[0] == sim and s[2] == delay for s in stalls), (sim, delay))
        self.assertEqual(self.played(), stalls)



# The catalog's fixture: a suite --sets farm run (sa0001: sets off and a)
# whose jobs a farm flush holds (F1, strims-sim), a second flush holding a
# copy of one of them with fewer tables (F2), a job it reused from another
# run that the cache never held (k9), and a local suite invocation's
# job (L).
F1, F2, L = "farm/sa0001-00+1more-aaaa/i-1/0", "farm/sa0001-00+1more-aaaa/i-2/0", "suite/wt/inv"
SHA = "0" * 40


class Catalog(unittest.TestCase):
    """catalog.py's index over a scratch cache and farm state: what it
    holds, how it normalizes the sims' tables, and that it does only what
    changed."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        series = {}
        for p in (*checkout(root),
                  mock.patch.object(verdict, "load_series",
                                    lambda pairs: {p: ({}, series[p]) for p in pairs if p in series}),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "target" / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        self.root = root
        now = time.time()
        recent = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now - 86400))
        old = time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(now - 9 * 86400))
        env = lambda exp, seed: {"STRIMS_SIM_EXP": exp, "STRIMS_SIM_SEED": str(seed), "STRIMS_SIM_TCP": "1"}
        row = lambda **kw: {"lag_p99_ms": 300, "meter_seeds": [1.5, 2], "arrivals": "trickle", "passed": True, **kw}
        self.put(F1, "runs", ["sim", "test", "cell", "arm", "seed", "env", "at", "row_json"], [
            ("k1", "strims-sim:scenarios", "t", "sa0001-00", "x", 1, json.dumps(env("off", 1)), recent,
             json.dumps(row())),
            ("k2", "strims-sim:scenarios", "t", "sa0001-00", "x", 1, json.dumps(env("a", 1)), recent,
             json.dumps(row(lag_p99_ms=200, extra_ms=5)))])
        self.put(F1, "viewers", ["viewer", "host", "name", "up", "hostile", "churned", "arrive_ms", "depart_ms"], [
            ("k1", 0, 7, "v0", -1, False, False, -5, None), ("k1", 1, 8, "v1", 100, False, True, 2000, 4000)])
        self.put(F1, "viewer_seconds", ["viewer", "sec", "lag_ms", "frames"],
                 [("k1", 0, s, 100 if s else -1, 1 if s else 0) for s in range(5)])
        self.put(F1, "events", ["t_us", "kind", "node", "peer", "value", "detail"], [
            ("k1", 1000, "parent_lost", 7, 8, -1, "abrupt"), ("k1", 2000, "join", -1, -1, 0, "from=3 to=4"),
            ("k1", 3000, "parent_noticed", 7, 8, 40, "")])
        self.put(F1, "seconds", ["sec", "present", "written"], [("k1", 0, 2, -1), ("k1", 1, 2, 10)])
        self.put(F1, "bins", ["bin", "t_ms", "seed_meter"], [("k1", 0, 0, -1), ("k1", 1, 100, 5)])
        (simdb.cache_root() / "cache" / F1 / "manifest.json").write_text(json.dumps(
            {"keys": ["k1", "k2"], "at": recent, "tables": {"runs": {"bytes": 1}, "reads": {"bytes": 99}}}))
        self.put(F2, "runs", ["sim", "test", "cell", "arm", "seed", "env", "at", "row_json"], [
            ("k1", "strims-sim:scenarios", "t", "sa0001-00", "x", 1, json.dumps(env("off", 1)), recent,
             json.dumps(row()))])
        (simdb.cache_root() / "cache" / F2 / "manifest.json").write_text(json.dumps({"keys": ["k1"], "at": old}))
        self.put(L, "runs", ["sim", "test", "cell", "arm", "seed", "env", "at", "row_json"], [
            ("k3", "strims-sim", "churn", "churn", "strims-sim-123", 2, "{}", recent,
             json.dumps({"lag_p99_ms": 400}))])
        # an older viewers file: node for its host, churned an integer
        self.put(L, "viewers", ["viewer", "node", "up", "churned", "arrive_ms", "depart_ms"], [
            ("k3", 0, 1, 50, 0, -1000, None), ("k3", 1, 2, 50, 1, 2500, 4000)])
        self.put(L, "viewer_seconds", ["viewer", "sec", "lag_ms", "frames"],
                 [("k3", v, s, -1 if v and not 2 <= s < 4 else 100, 0 if v and not 2 <= s < 4 else 3)
                  for v in (0, 1) for s in range(6)])
        for src, k in ((F1, "k1"), (F1, "k2"), (L, "k3")):
            series[(src, k)] = audience(40, [(None, None)] * 3)
        runs = root / "target" / "sim-suite" / "farm-runs"
        runs.mkdir(parents=True)
        (runs / "sa0001.json").write_text(json.dumps({
            "cells": [{"name": "star", "sim": "strims-sim", "test": "t", "why": "w", "env": {"STRIMS_SIM_TCP": "1"}}],
            "knob": {"strims-sim": "STRIMS_SIM_EXP"}, "values": ["off", "a"],
            "arms": {"x": SHA}, "seeds": [1], "base": "off", "question": "does a help?"}))
        farm = root / "target" / "sim-farm"
        (farm / "sa0001-00").mkdir(parents=True)
        (farm / "sa0001-00+1more-aaaa").mkdir(parents=True)
        (farm / "sa0001-00+1more-aaaa" / "matrices").write_text("sa0001-00\n")
        line = lambda i, k, exp, src, **kw: {"id": i, "key": k, "arm": "x", "commit": SHA, "binary": "b" * 64,
                                             "test": "t", "env": env(exp, 1), "src": src, "at": recent,
                                             "wall_s": 1.5, "max_rss_mb": 10, "row": {"trace": f"tr-{k}"},
                                             "reads": True, **kw}
        (farm / "sa0001-00" / "index.jsonl").write_text("".join(json.dumps(r) + "\n" for r in [
            line("i1", "k1", "off", F1), line("i2", "k2", "a", F1),
            line("i9", "k9", "a", "farm/sb0001-00+0more-bbbb/i-1/0", **{"from": "sb0001-00", "at": old})]))
        (farm / "costs.jsonl").write_text(json.dumps({"run": "sa0001-00+1more-aaaa", "kind": "test", "cost": 1.5,
                                                      "jobs": 2}) + "\n")
        (farm / "s3.jsonl").write_text(json.dumps({"run": "sa0001-00+1more-aaaa", "at": recent, "source": "pull",
                                                   "region": "r", "tier1": {}, "tier2": {"pull_get": 10},
                                                   "up": {}, "down": {"internet": 1 << 30}, "stored": 0}) + "\n")

    def put(self, src, table, cols, rows):
        d = simdb.cache_root() / "cache" / src
        d.mkdir(parents=True, exist_ok=True)
        lit = lambda x: "NULL" if x is None else simdb.q(x) if isinstance(x, str) else repr(x)
        vals = ", ".join("(" + ", ".join(lit(x) for x in (r[0], src, *r[1:])) + ")" for r in rows)
        names = ", ".join(simdb.ident(c) for c in ["k", "src", *cols])
        simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t({names})) "
                   f"TO {simdb.q(d / f'{table}.parquet')} (FORMAT parquet);", out=None)

    def index(self):
        return catalog.index(workers=0, quiet=True)

    def q(self, sql):
        return catalog.lake(sql + ";")

    def test_jobs_one_a_key_from_its_canonical_source(self):
        self.index()
        jobs = {r["k"]: r for r in self.q("SELECT k, sim, seed, src, cached, commit, exp, transport, env['STRIMS_SIM_EXP'] "
                                          "AS e, trace FROM jobs")}
        self.assertEqual(set(jobs), {"k1", "k2", "k3", "k9"})
        # the copy holding the most tables
        self.assertEqual(jobs["k1"]["src"], F1)
        self.assertEqual((jobs["k1"]["sim"], jobs["k3"]["sim"]), ("strims-sim", "strims-sim"))
        self.assertEqual((jobs["k2"]["exp"], jobs["k2"]["e"], jobs["k2"]["transport"]), ("a", "a", "tcp"))
        self.assertEqual((jobs["k9"]["cached"], jobs["k9"]["sim"], jobs["k9"]["seed"], jobs["k9"]["trace"]),
                         (False, "strims-sim", 1, "tr-k9"))

    def test_experiments_name_their_arms_and_reused_jobs(self):
        self.index()
        got = {(r["cell"], r["arm_label"], r["k"]): (r["ref"], r["knob_value"], r["reused"]) for r in self.q(
            "SELECT * FROM experiment_jobs WHERE exp_id = 'sa0001'")}
        self.assertEqual(got, {("star", "off", "k1"): ("x", "off", False), ("star", "a", "k2"): ("x", "a", False),
                               ("star", "a", "k9"): ("x", "a", True)})
        e = {r["exp_id"]: r for r in self.q("SELECT exp_id, kind, question, base, sets, cost_usd, s3_usd, jobs FROM experiments")}
        self.assertEqual((e["sa0001"]["question"], e["sa0001"]["base"], e["sa0001"]["sets"]), ("does a help?", "off", ["off", "a"]))
        # the run's only matrix: its whole cost, and the pull's GiB out
        self.assertAlmostEqual(e["sa0001"]["cost_usd"], 1.5)
        self.assertGreater(e["sa0001"]["s3_usd"], 0.08)
        self.assertEqual((e[L]["kind"], e[L]["jobs"]), ("local", 1))

    def test_a_key_two_arms_share_costs_one_run(self):
        # The run's other matrix, sr0001-00, a --refs run's: arms old and
        # new, whose binaries are the same bytes, share kA and kB, which
        # the run ran once, under old, new's lines naming their own matrix.
        farm = self.root / "target" / "sim-farm"
        (farm / "sa0001-00+1more-aaaa" / "matrices").write_text("sa0001-00\nsr0001-00\n")
        (self.root / "target" / "sim-suite" / "farm-runs" / "sr0001.json").write_text(json.dumps({
            "cells": [{"name": "star", "sim": "strims-sim", "test": "t", "why": "w", "env": {"STRIMS_SIM_TCP": "1"}}],
            "arms": {"old": SHA, "new": SHA}, "seeds": [1, 2], "base": "old", "question": "same bytes?"}))
        env = lambda seed: {"STRIMS_SIM_SEED": str(seed), "STRIMS_SIM_TCP": "1"}  # noqa: E731
        line = lambda i, k, arm, seed, **kw: {"id": i, "key": k, "arm": arm, "commit": SHA, "binary": "b" * 64,  # noqa: E731
                                              "test": "t", "env": env(seed), "src": F1, "wall_s": 1.5, "row": {}, **kw}
        (farm / "sr0001-00").mkdir()
        (farm / "sr0001-00" / "index.jsonl").write_text("".join(json.dumps(r) + "\n" for r in [
            line("o1", "kA", "old", 1), line("n1", "kA", "new", 1, **{"from": "sr0001-00"}),
            line("o2", "kB", "old", 2), line("n2", "kB", "new", 2, **{"from": "sr0001-00"})]))
        self.index()
        got = {(r["arm_label"], r["k"]): r["reused"] for r in self.q(
            "SELECT * FROM experiment_jobs WHERE exp_id = 'sr0001'")}
        self.assertEqual(got, {("old", "kA"): False, ("new", "kA"): False, ("old", "kB"): False,
                               ("new", "kB"): False})
        # Each matrix ran two of the run's four runs (sa0001-00's k9 is
        # another run's): half its cost each.
        e = {r["exp_id"]: r for r in self.q("SELECT exp_id, cost_usd, s3_usd FROM experiments")}
        self.assertAlmostEqual(e["sr0001"]["cost_usd"], 0.75)
        self.assertAlmostEqual(e["sa0001"]["cost_usd"], 0.75)
        self.assertAlmostEqual(e["sr0001"]["s3_usd"], e["sa0001"]["s3_usd"])

    def test_the_views_normalize_the_sims_tables(self):
        self.index()
        v = {(r["k"], r["viewer"]): r for r in self.q("SELECT * FROM viewers")}
        self.assertEqual((v[("k1", 0)]["node"], v[("k1", 0)]["arrive_ms"], v[("k1", 0)]["up"]), (7, None, None))
        self.assertEqual((v[("k1", 1)]["arrive_ms"], v[("k1", 1)]["churned"]), (2000, True))
        # the older file's integer churned, as a BOOLEAN; its -1000 start NULL
        self.assertEqual((v[("k3", 0)]["node"], v[("k3", 0)]["churned"], v[("k3", 0)]["arrive_ms"]), (1, False, None))
        self.assertEqual(v[("k3", 1)]["churned"], True)
        # viewer 1 of k3 was there from 2.5 s to 4 s: seconds 2 and 3, which its stay overlaps
        secs = [(r["viewer"], r["sec"], r["lag_ms"]) for r in self.q(
            "SELECT viewer, sec, lag_ms FROM viewer_seconds WHERE k = 'k3' ORDER BY ALL")]
        self.assertEqual([s for v_, s, _ in secs if v_ == 1], [2, 3])
        self.assertEqual(len([s for v_, s, _ in secs if v_ == 0]), 6)
        self.assertEqual(self.q("SELECT lag_ms FROM viewer_seconds WHERE k = 'k1' AND sec = 0")[0]["lag_ms"], None)
        ev = {r["kind"]: r for r in self.q("SELECT * FROM events WHERE k = 'k1'")}
        self.assertEqual((ev["join"]["node"], ev["join"]["peer"], ev["join"]["t_ms"]), (None, None, 2.0))
        self.assertEqual(ev["join"]["detail_map"], {"from": "3", "to": "4"})
        # strims-sim's retired parent_lost of -1, a loss never noticed, reads as a parent_departed with no value;
        # its replacements' values are their own.
        self.assertEqual((ev["parent_departed"]["written_kind"], ev["parent_departed"]["value"],
                          ev["parent_departed"]["detail_map"]), ("parent_lost", None, None))
        self.assertEqual((ev["parent_noticed"]["written_kind"], ev["parent_noticed"]["value"]), ("parent_noticed", 40))
        self.assertEqual([r["written"] for r in self.q("SELECT written FROM seconds ORDER BY sec")], [None, 10])
        self.assertEqual([r["seed_meter"] for r in self.q("SELECT seed_meter FROM bins ORDER BY bin")], [None, 5])

    def test_rows_are_wide_and_typed(self):
        self.index()
        types = {r["column_name"]: r["data_type"] for r in self.q(
            "SELECT column_name, data_type FROM information_schema.columns WHERE table_name = 'rows' "
            "AND table_schema = 'main'")}
        self.assertEqual((types["lag_p99_ms"], types["meter_seeds"], types["arrivals"], types["passed"]),
                         ("DOUBLE", "DOUBLE[]", "VARCHAR", "BOOLEAN"))
        got = {r["k"]: r for r in self.q("SELECT k, lag_p99_ms, extra_ms, meter_seeds FROM rows")}
        self.assertEqual((got["k2"]["lag_p99_ms"], got["k2"]["extra_ms"], got["k1"]["extra_ms"]), (200, 5, None))
        self.assertEqual(got["k1"]["meter_seeds"], [1.5, 2.0])

    def test_verdicts_are_derives(self):
        self.index()
        got = {r["k"]: r for r in self.q("SELECT * FROM verdicts")}
        self.assertEqual(set(got), {"k1", "k2", "k3"})
        version = self.q("SELECT value FROM catalog_info WHERE key = 'verdict_version'")[0]["value"]
        self.assertEqual(version, simdb.derived_version())
        want = simdb.judge(audience(40, [(None, None)] * 3))
        self.assertEqual((got["k1"]["collapsed"], got["k1"]["unhealthy"]),
                         (verdict.collapsed(want[0]), want[0]["unhealthy"]))
        self.assertEqual(got["k1"]["flaps_per_min"], want[1]["flaps_per_min"])

    def test_availability(self):
        self.index()
        got = {(r["k"], r["tbl"]): (r["state"], r["src"]) for r in self.q("SELECT * FROM availability")}
        self.assertEqual(got[("k1", "viewers")], ("local", F1))
        # F1 lists reads it doesn't hold, uploaded a day ago
        self.assertEqual(got[("k1", "reads")], ("s3", F1))
        # k9's flush, never pulled, ran nine days ago
        self.assertEqual(got[("k9", "sim-data")][0], "expired")

    def test_it_does_only_what_changed(self):
        self.assertTrue(self.index().changed)
        self.assertFalse(self.index().changed)
        linked = [Path(r["path"]) for r in self.q("SELECT path FROM raw.files WHERE how = 'linked'")]
        self.assertTrue(linked and all(p.stat().st_nlink == 2 for p in linked))
        self.assertEqual(self.q("SELECT how FROM raw.files WHERE src = 'suite/wt/inv' AND tbl = 'viewers'")[0]["how"],
                         "converted")
        # a new source: only its files are added
        self.put("suite/wt/inv2", "runs", ["sim", "test", "cell", "arm", "seed", "env", "row_json"],
                 [("k4", "strims-sim", "churn", "churn", "a", 1, "{}", "{}")])
        got = self.index()
        self.assertEqual((got.new, got.gone), (1, 0))
        self.assertIn("k4", {r["k"] for r in self.q("SELECT k FROM jobs")})
        # a job of the ppspp sim's the index leaves out
        self.put("suite/wt/inv3", "runs", ["sim", "test", "cell", "arm", "seed", "env", "row_json"],
                 [("k5", "ppspp", "churn", "churn", "a", 1, "{}", "{}")])
        got = self.index()
        self.assertEqual((got.new, got.gone), (1, 0))
        self.assertNotIn("k5", {r["k"] for r in self.q("SELECT k FROM jobs")})
        # the cache drops F1 (simdb.py gc): its jobs' tables leave, F2's copy of k1 stands in
        shutil.rmtree(simdb.cache_root() / "cache" / F1)
        got = self.index()
        self.assertEqual(got.gone, 6)
        self.assertEqual(self.q("SELECT src FROM jobs WHERE k = 'k1'")[0]["src"], F2)
        self.assertEqual(self.q("SELECT count(*) AS n FROM viewers WHERE k LIKE 'k1'")[0]["n"], 0)
        self.assertEqual(self.q("SELECT count(*) AS n FROM raw.verdicts WHERE src = " + simdb.q(F1))[0]["n"], 0)
        catalog.compact(keep_minutes=0)
        self.assertFalse(any(p.exists() for p in linked if F1.replace("/", "~") in p.name))

    def test_the_catalogs_describe_it(self):
        self.index()
        m = {(r["tbl"], r["name"]): r for r in self.q("SELECT * FROM metric_catalog")}
        self.assertEqual((m[("suite", "lag_p99_ms")]["better"], m[("suite", "lag_p99_ms")]["kind"],
                          m[("suite", "lag_p99_ms")]["tie"]), ("lower", "ratio", 0.05))
        self.assertEqual(m[("suite", "lag_p99_ms")]["fields"], ["lag_p99_ms", "lag_p99_ms_honest"])
        self.assertEqual((m[("rows", "lag_p99_ms")]["better"], m[("rows", "lag_p99_ms")]["sims"]),
                         ("lower", ["strims-sim"]))
        self.assertTrue(m[("viewer_seconds", "lag_ms")]["description"])
        t = {r["name"] for r in self.q("SELECT name FROM table_catalog")}
        self.assertTrue({"jobs", "experiments", "viewers", "verdicts", "availability"} <= t)
        kinds = {(r["sim"], r["kind"]) for r in self.q("SELECT sim, kind FROM event_kinds")}
        self.assertTrue({("strims-sim", "join"), ("strims-sim", "parent_lost"), ("strims-sim", "parent_noticed"),
                         ("derived", "start")} <= kinds)
        retired = {r["sim"]: r["replaced_by"] for r in self.q("SELECT sim, replaced_by FROM event_kinds "
                                                              "WHERE kind = 'parent_lost'")}
        self.assertEqual(retired, {"strims-sim": "parent_noticed"})


if __name__ == "__main__":
    bootstrap.engine(__file__)
    unittest.main()
