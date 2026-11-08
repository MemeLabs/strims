#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's selections over a scratch cache, suite indexes and farm state,
as two farm runs of several suite tiers leave them (an A/B and its
confirmation, each a browsers and a net tier, their cells sharing names,
the A/B reusing rows of the confirmation's):

  python3 hack/sim-db/simdb_test.py

Needs the pinned DuckDB CLI (simdb.py duckdb fetches it)."""

import argparse
import importlib.util
import io
import json
import os
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


if __name__ == "__main__":
    unittest.main()
