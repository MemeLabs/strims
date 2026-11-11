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
(SamplesPull); the derived tables' columns and versions (Derived);
macros.sql's playback() against the sims' own stalls (Playback); and
catalog.py's index of a scratch cache and farm state (Catalog):

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
import time
import unittest
from pathlib import Path
from unittest import mock

spec = importlib.util.spec_from_file_location("simdb", Path(__file__).with_name("simdb.py"))
simdb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(simdb)
spec = importlib.util.spec_from_file_location("suite", Path(__file__).parents[1] / "sim-suite" / "suite.py")
suite = importlib.util.module_from_spec(spec)
spec.loader.exec_module(suite)
# suite.py's simdb is this one, so the tests' patches reach it.
suite._simdb = simdb
spec = importlib.util.spec_from_file_location("catalog", Path(__file__).with_name("catalog.py"))
catalog = importlib.util.module_from_spec(spec)
spec.loader.exec_module(catalog)
catalog._simdb = simdb

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


# Suite farm runs, as suite.py's `farm` leaves them: a --sets run (the
# knob its index names, set apart from SIM_EXP for strims-sim), a --refs
# run whose --env sets the default knob, one ref named as a set ("off"),
# and two runs without an index: one of sets (the matrices' lone arm, x,
# the default knob an axis), and one of refs whose env gives each the
# other's name.
LABEL_CELLS = [{"name": "star-3-5", "sim": "strims-sim", "test": "calibration_star"},
               {"name": "org-star-7-5", "sim": "ppspp", "test": "sim_job"}]
SETS_KNOB = {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_KNOWN_UPLINK"}
LABEL_INDEXES = {"sd0001": {"cells": LABEL_CELLS, "knob": SETS_KNOB, "values": ["off", "seed", "1"]},
                 "sd0002": {"cells": LABEL_CELLS}}
# run: [(arm, {sim: (knob, value)})]
LABEL_ARMS = {
    "sd0001": [("x", {"ppspp": ("SIM_EXP", v), "strims-sim": ("STRIMS_SIM_KNOWN_UPLINK", v)})
               for v in ("off", "seed", "1")],
    "sd0002": [(r, {"ppspp": ("SIM_EXP", "off"), "strims-sim": ("STRIMS_SIM_EXP", "off")}) for r in ("main", "off")],
    "sd0003": [("x", {"ppspp": ("SIM_EXP", v), "strims-sim": ("STRIMS_SIM_EXP", v)}) for v in ("off", "a")],
    "sd0004": [(r, {"ppspp": ("SIM_EXP", v), "strims-sim": ("STRIMS_SIM_EXP", v)})
               for r, v in (("off", "on"), ("on", "off"))],
}
LABEL_SEEDS = (1, 2)


class ArmLabels(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.root = root = Path(tmp.name)
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(suite, "farm_root", lambda: root),
                  mock.patch.object(simdb, "pull_sources", lambda s, reads=False: []),
                  mock.patch.object(simdb, "rows", lambda pairs, series=False: {(s, k): {"key": k} for s, k in pairs}),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
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
                        e = {knob: value, "SIM_SEED" if c["sim"] == "ppspp" else "STRIMS_SIM_SEED": str(s)}
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
        return {arm: {at: r["key"] for at, r in rs.items()} for arm, rs in rows.items()}

    def score(self, arg, knob=None):
        """`score`'s rows of RUN:ARM `arg`, {(cell, seed): key}."""
        got = {}
        args = argparse.Namespace(a=arg, b=arg, knob=knob, only=None, rank="lag", all=False)
        with mock.patch.object(suite, "scorecard", lambda cells, a_rows, b_rows, *_: got.update(a_rows)):
            suite.score(args, LABEL_CELLS)
        return {at: r["key"] for at, r in got.items()}

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
            self.score("sd0001:off", knob="SIM_EXP,STRIMS_SIM_EXP")
        self.assertIn("index's knob", str(cm.exception.code))
        self.assertEqual(self.score("sd0001:off", knob="SIM_EXP,STRIMS_SIM_KNOWN_UPLINK"), got["off"])

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
                 suite.cell("large", suite.PPSPP, "sim_job", {"SIM_PEERS": "200"}, 3, "a large swarm")]
        args = argparse.Namespace(refs=None, sets="seed;1", ref="HEAD", no_off=False, knob="SIM_EXP,STRIMS_SIM_EXP",
                                  first_seed=2001, seeds=3, env=[], no_run=True, farm_args="", against=None,
                                  question="does seed help?", all=False, rank="lag")
        sha = "a" * 40
        with mock.patch.object(suite, "git", lambda *a, **kw: sha), \
                mock.patch.object(suite, "peak_gib", lambda r: {"star-3-5": 0.5}), \
                mock.patch.object(suite, "WORK", self.root / "work"), \
                mock.patch.object(suite, "print_arm_score", lambda *a: None), \
                mock.patch.object(suite, "scorecard", lambda *a, **kw: None):
            suite.farm(args, cells)
        [path] = (self.root / "target" / "sim-suite" / "farm-runs" / "pending").glob("*.json")
        idx = simdb.suite_index(path.stem)
        self.assertEqual(idx["cells"], [
            {"name": "star-3-5", "sim": "strims-sim", "test": "calibration_star", "why": "3 viewers",
             "env": {"STRIMS_SIM_CAL_VIEWERS": "3"}, "mem_gib": 1.0},
            {"name": "large", "sim": "ppspp", "test": "sim_job", "why": "a large swarm",
             "env": {"SIM_PEERS": "200"}, "mem_gib": 3}])
        self.assertEqual((idx["arms"], idx["seeds"], idx["knob"], idx["values"]),
                         ({"x": sha}, [2001, 2002, 2003], {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"},
                          ["off", "seed", "1"]))
        self.assertEqual((idx["base"], idx["question"]), ("off", "does seed help?"))
        self.assertEqual(simdb.suite_cells([f"{path.stem}-01"]), {f"{path.stem}-01": (f"large@{path.stem}", "SIM_EXP")})

    def test_a_label_holding_two_jobs_is_refused(self):
        with self.assertRaises(SystemExit) as cm:
            suite.farm_rows("sd0001", LABEL_CELLS, None)
        self.assertIn("arm x holds two jobs at one cell and seed", str(cm.exception.code))

    def test_simdb_names_the_arms_as_suite_does(self):
        v, _ = simdb.views(argparse.Namespace(src=None, cell=None, arm=None, seed=None), derived=False)
        views = {r["k"]: (r["cell"], r["arm"]) for r in simdb.duck(v + "\nSELECT k, cell, arm FROM runs;")}
        for run in LABEL_INDEXES:
            for arm, rows in self.farm_rows(run).items():
                for (cell, _), k in rows.items():
                    self.assertEqual(views[k], (f"{cell}@{run}", arm), k)
        self.assertEqual(simdb.arm_label(SETS_KNOB, "strims-sim", "x", {"STRIMS_SIM_KNOWN_UPLINK": "1"}), "1")
        self.assertEqual(simdb.arm_label(None, "strims-sim", "main", {"STRIMS_SIM_EXP": "off"}), "main")


class Canon(unittest.TestCase):
    """Which of a key's copies the views take its tables from: the one
    holding the most tables, then the later flush, then the later seq."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
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
        v, _ = simdb.views(argparse.Namespace(src=None, cell=None, arm=None, seed=None), derived=False)
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
        v, _ = simdb.views(self.args(src=src))
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
        self.assertEqual(self.aligned("depart", "reads", "run", nth=0, before=0, after=1),
                         {("c", 0): (2, 15.5, 17.5, 19.5)})
        self.assertEqual(self.aligned("depart", "lag_p50", "node", before=0, after=1),
                         {("c", 0): (2, 120.0, 200.0, 280.0)})
        self.assertEqual(self.aligned("depart", "lag_p50", "run", before=0, after=1, detail="abrupt"),
                         {("c", 0): (1, 200.0, 200.0, 200.0)})

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
                         {("c", 0): (3, 12.0, 20.0, 20.0)})
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
        self.assertEqual(self.aligned("first_read", "forecast_err_p50", "run", before=0, after=1),
                         {("c", 0): (1, -35.0, -35.0, -35.0)})
        self.assertEqual(self.aligned("first_read", "forecast_abs_err_p90", "run", before=0, after=1)[("c", 0)][2],
                         qc([50, 120], 0.9))
        # per node, node 2's sample after its own first read counts too
        got = self.aligned("first_read", "forecast_n", "node", before=0, after=1)
        self.assertEqual({k: (n, p50) for k, (n, _, p50, _) in got.items()}, {("c", 0): (2, 1.5)})
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
    """analyze.py's verdict counts each viewer only while it is there and
    past its own startup (analyze.presence)."""

    a = simdb.analyze()

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
        # its first frame plus the grace, so trees forming around arrivals
        # read as nothing, where counting them from their arrival reads the
        # first seconds' trees as failing.
        stays = [(k + 0.5, None) for k in range(20)]
        for sim in ("ppspp", "strims-sim"):
            v, collapsed = self.verdict(audience(60, stays, sim))
            self.assertTrue(all(v["healthy"]), sim)
            self.assertFalse(collapsed, sim)
            self.assertEqual(v["startup"], 0, sim)
            first = [since for _, since, _ in self.a.presence(audience(60, stays, sim))]
            self.assertEqual(first[:2], [1 + self.a.JOIN_GRACE_SECS, 2 + self.a.JOIN_GRACE_SECS], sim)
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
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(simdb, "load_series", lambda pairs: {p: series[p] for p in pairs}),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
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
        v, have = simdb.views(argparse.Namespace(src=None, cell=None, arm=None, seed=None))
        return simdb.duck(v + "\n" + query + ";"), have

    def test_a_table_holds_its_columns_alone(self):
        path = simdb.cache_root() / "derived" / "t" / "v=abc" / "x.parquet"
        simdb.write_table(path, simdb.DERIVED["episodes"], [["k", "s", "collapse", 1, 9, 8]])
        self.assertEqual(self.columns(path), list(simdb.DERIVED["episodes"]))

    def test_verdicts_carry_the_startup(self):
        got, _ = self.view("SELECT k, startup_s, collapsed FROM verdicts ORDER BY k")
        a = simdb.analyze()
        self.assertEqual({r["k"]: r["startup_s"] for r in got}, {"kt0": a.STARTUP_SECS, "ktr": 0})
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
        a = simdb.analyze()
        name = self.SRC.replace("/", "~") + ".parquet"
        old = {c: t for c, t in simdb.VERDICT_COLS.items() if c != "startup_s"}
        made = [[k, self.SRC] + [None] * (len(old) - 2) for k in self.STAYS]
        for t, cols, rows in [*((t, c, []) for t, c in simdb.DERIVED.items()), ("verdicts", old, made)]:
            simdb.write_table(simdb.cache_root() / "derived" / t / f"v={a.VERDICT_VERSION}" / name, cols, rows)
        got, _ = self.view("SELECT k, startup_s FROM verdicts ORDER BY k")
        self.assertEqual([(r["k"], r["startup_s"]) for r in got], [("kt0", a.STARTUP_SECS), ("ktr", 0)])
        now = simdb.derived_version()
        self.addCleanup(simdb.derived_version.cache_clear)
        simdb.derived_version.cache_clear()
        with mock.patch.object(simdb, "VERDICT_COLS", old):
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
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(simdb, "derive", lambda srcs: None),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
            p.start()
            self.addCleanup(p.stop)
        a = simdb.analyze()
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
        v, _ = simdb.views(argparse.Namespace(src=None, cell=None, arm=None, seed=None))
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
# ppspp job (L).
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
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.object(simdb, "load_series",
                                    lambda pairs: {p: ({}, series[p]) for p in pairs if p in series}),
                  mock.patch.object(catalog, "_simdb", simdb),
                  mock.patch.object(sys, "stderr", io.StringIO()),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
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
            ("k1", 1000, "parent_lost", 7, 8, -1, "abrupt"), ("k1", 2000, "join", -1, -1, 0, "from=3 to=4")])
        self.put(F1, "seconds", ["sec", "present", "written"], [("k1", 0, 2, -1), ("k1", 1, 2, 10)])
        self.put(F1, "bins", ["bin", "t_ms", "seed_meter"], [("k1", 0, 0, -1), ("k1", 1, 100, 5)])
        (simdb.cache_root() / "cache" / F1 / "manifest.json").write_text(json.dumps(
            {"keys": ["k1", "k2"], "at": recent, "tables": {"runs": {"bytes": 1}, "reads": {"bytes": 99}}}))
        self.put(F2, "runs", ["sim", "test", "cell", "arm", "seed", "env", "at", "row_json"], [
            ("k1", "strims-sim:scenarios", "t", "sa0001-00", "x", 1, json.dumps(env("off", 1)), recent,
             json.dumps(row()))])
        (simdb.cache_root() / "cache" / F2 / "manifest.json").write_text(json.dumps({"keys": ["k1"], "at": old}))
        self.put(L, "runs", ["sim", "test", "cell", "arm", "seed", "env", "at", "row_json"], [
            ("k3", "ppspp", "churn", "churn", "ppspp-123", 2, "{}", recent, json.dumps({"lag_p99_ms": 400}))])
        # an old ppspp viewers file: churned an integer
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
            "knob": {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"}, "values": ["off", "a"],
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
        # farm.py's prices, which the catalog reads from it
        (root / "hack" / "sim-farm").mkdir(parents=True)
        (root / "hack" / "sim-farm" / "farm.py").write_text(
            "S3_TIER1 = 0.0055 / 1000\nS3_TIER2 = 0.0044 / 10000\nS3_STORAGE_GB_MONTH = 0.026\nS3_KEPT_DAYS = 7\n"
            "S3_INTER_REGION_GB = 0.02\nS3_INTERNET_GB = 0.09\nGB = 1 << 30\n")
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
        return catalog.index(workers=1, quiet=True)

    def q(self, sql):
        return catalog.lake(sql + ";")

    def test_jobs_one_a_key_from_its_canonical_source(self):
        self.index()
        jobs = {r["k"]: r for r in self.q("SELECT k, sim, seed, src, cached, commit, exp, transport, env['STRIMS_SIM_EXP'] "
                                          "AS e, trace FROM jobs")}
        self.assertEqual(set(jobs), {"k1", "k2", "k3", "k9"})
        # the copy holding the most tables
        self.assertEqual(jobs["k1"]["src"], F1)
        self.assertEqual((jobs["k1"]["sim"], jobs["k3"]["sim"]), ("strims-sim", "ppspp"))
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

    def test_the_views_normalize_the_sims_tables(self):
        self.index()
        v = {(r["k"], r["viewer"]): r for r in self.q("SELECT * FROM viewers")}
        self.assertEqual((v[("k1", 0)]["node"], v[("k1", 0)]["arrive_ms"], v[("k1", 0)]["up"]), (7, None, None))
        self.assertEqual((v[("k1", 1)]["arrive_ms"], v[("k1", 1)]["churned"]), (2000, True))
        # the ppspp file's integer churned, as a BOOLEAN; its -1000 start NULL
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
        self.assertEqual((ev["parent_lost"]["value"], ev["parent_lost"]["detail_map"]), (None, None))
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
                         (simdb.analyze().collapsed(want[0]), want[0]["unhealthy"]))
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
        self.assertTrue(self.index()["changed"])
        self.assertFalse(self.index()["changed"])
        linked = [Path(r["path"]) for r in self.q("SELECT path FROM raw.files WHERE how = 'linked'")]
        self.assertTrue(linked and all(p.stat().st_nlink == 2 for p in linked))
        self.assertEqual(self.q("SELECT how FROM raw.files WHERE src = 'suite/wt/inv' AND tbl = 'viewers'")[0]["how"],
                         "converted")
        # a new source: only its files are added
        self.put("suite/wt/inv2", "runs", ["sim", "test", "cell", "arm", "seed", "env", "row_json"],
                 [("k4", "ppspp", "churn", "churn", "a", 1, "{}", "{}")])
        got = self.index()
        self.assertEqual((got["new"], got["gone"]), (1, 0))
        self.assertIn("k4", {r["k"] for r in self.q("SELECT k FROM jobs")})
        # the cache drops F1 (simdb.py gc): its jobs' tables leave, F2's copy of k1 stands in
        shutil.rmtree(simdb.cache_root() / "cache" / F1)
        got = self.index()
        self.assertEqual(got["gone"], 6)
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
                         ("lower", ["ppspp", "strims-sim"]))
        self.assertTrue(m[("viewer_seconds", "lag_ms")]["description"])
        t = {r["name"] for r in self.q("SELECT name FROM table_catalog")}
        self.assertTrue({"jobs", "experiments", "viewers", "verdicts", "availability"} <= t)
        kinds = {(r["sim"], r["kind"]) for r in self.q("SELECT sim, kind FROM event_kinds")}
        self.assertTrue({("strims-sim", "join"), ("strims-sim", "parent_lost"), ("derived", "start")} <= kinds)


if __name__ == "__main__":
    unittest.main()
