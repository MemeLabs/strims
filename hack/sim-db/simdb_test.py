#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simdb.py's selections over a scratch cache and suite indexes, as two
farm runs of several suite tiers leave them (an A/B and its confirmation,
each a browsers and a net tier, their cells sharing names):

  python3 hack/sim-db/simdb_test.py

Needs the pinned DuckDB CLI (simdb.py duckdb fetches it)."""

import argparse
import importlib.util
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest import mock

spec = importlib.util.spec_from_file_location("simdb", Path(__file__).with_name("simdb.py"))
simdb = importlib.util.module_from_spec(spec)
spec.loader.exec_module(simdb)

A, B = "farm/sa0001-00+2more-aaaa", "farm/sb0001-00+2more-bbbb"
INDEXES = {
    "sa0001": {"cells": [{"name": "star-3-5", "sim": "strims-sim"}, {"name": "halve-7-10", "sim": "strims-sim"}]},
    "sa0002": {"cells": [{"name": "org-star-7-5", "sim": "ppspp"}],
               "knob": {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"}, "values": ["off", "a"]},
    "sb0001": {"cells": [{"name": "star-3-5", "sim": "strims-sim"}, {"name": "halve-7-10", "sim": "strims-sim"}]},
}
# Kept apart by suite.py until its run started.
PENDING = {"sb0002": {"cells": [{"name": "org-star-7-5", "sim": "ppspp"}]}}
# source: (k, matrix, arm, env)
STORED = {
    f"{A}/i-1/0": [("k1", "sa0001-00", "main", {}), ("k2", "sa0001-00", "fix", {}),
                   ("k3", "sa0001-01", "main", {}), ("k4", "sa0002-00", "x", {"SIM_EXP": "off"}),
                   ("k5", "sa0002-00", "x", {"SIM_EXP": "a"}),
                   # a verify copy of a job the farm reused from B's run
                   ("kr", "sa0001-01", "main", {})],
    # a second instance's copy of k2
    f"{A}/i-2/0": [("k2", "sa0001-00", "fix", {})],
    f"{B}/i-1/0": [("k6", "sb0001-00", "main", {}), ("k7", "sb0002-00", "x", {}),
                   ("kr", "sb0001-01", "main", {})],
    "suite/wt/inv": [("k8", "star-3-5", "strims-sim-0123", {})],
}


class Selection(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        for p in (mock.patch.object(simdb, "main_checkout", lambda: root),
                  mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != "SIM_DB_ROOT"},
                                  clear=True)):
            p.start()
            self.addCleanup(p.stop)
        runs = root / "target" / "sim-suite" / "farm-runs"
        (runs / "pending").mkdir(parents=True)
        for d, idx in ((runs, INDEXES), (runs / "pending", PENDING)):
            for run, x in idx.items():
                (d / f"{run}.json").write_text(json.dumps(x))
        q = simdb.q
        for src, rows in STORED.items():
            path = simdb.cache_root() / "cache" / src / "runs.parquet"
            path.parent.mkdir(parents=True)
            vals = ", ".join(f"({q(k)}, {q(src)}, {q(m)}, {q(arm)}, 1, {q(json.dumps(env))})"
                             for k, m, arm, env in rows)
            simdb.duck(f"COPY (SELECT * FROM (VALUES {vals}) t(k, src, cell, arm, seed, env)) TO {q(path)} "
                       "(FORMAT parquet);", out=None)

    def select(self, src=None, cell=None, sql="SELECT src, k, cell, arm FROM runs"):
        v, _ = simdb.views(argparse.Namespace(src=src, cell=cell, arm=None, seed=None), derived=False)
        return simdb.duck(v + "\n" + sql + ";")

    def runs(self, src=None, cell=None):
        return {(r["src"], r["k"]): (r["cell"], r["arm"]) for r in self.select(src, cell)}

    def test_src_keeps_every_run_and_its_name(self):
        every = self.runs()
        farm = [s for s in STORED if s.startswith("farm/")]
        for src, srcs in (([f"{A}*"], [f"{A}/i-1/0", f"{A}/i-2/0"]), ([f"{B}*"], [f"{B}/i-1/0"]),
                          ([f"{A}/i-1/0"], [f"{A}/i-1/0"]), (["farm/*"], farm), (["suite/*"], ["suite/wt/inv"]),
                          ([f"{A}*", f"{B}*"], farm)):
            stored = {(k, m) for s in srcs for k, m, _, _ in STORED[s]}
            got = self.select(src, sql="SELECT k, matrix FROM runs")
            self.assertEqual({(r["k"], r["matrix"]) for r in got}, stored, src)
            for key, named in self.runs(src).items():
                if key in every:
                    self.assertEqual(named, every[key], (src, key))

    def test_cells_are_named_by_their_suite_run(self):
        names = {c for c, _ in self.runs().values()}
        self.assertEqual(names, {"star-3-5@sa0001", "halve-7-10@sa0001", "org-star-7-5@sa0002",
                                 "star-3-5@sb0001", "halve-7-10@sb0001", "org-star-7-5@sb0002", "star-3-5"})
        self.assertEqual({c for c, _ in self.runs([f"{A}*"]).values()},
                         {"star-3-5@sa0001", "halve-7-10@sa0001", "org-star-7-5@sa0002"})

    def test_cell_glob_with_src(self):
        got = self.runs([f"{A}*"], ["org-star-7-5@sa0002"])
        self.assertEqual(sorted(got.values()), [("org-star-7-5@sa0002", "a"), ("org-star-7-5@sa0002", "off")])
        self.assertEqual(self.runs([f"{A}*"], ["org-*"]), got)
        self.assertEqual(self.runs(None, ["*@sa0002"]), got)
        self.assertEqual({c for c, _ in self.runs(None, ["star-3-5"]).values()},
                         {"star-3-5@sa0001", "star-3-5@sb0001", "star-3-5"})
        self.assertEqual({c for c, _ in self.runs([f"{B}*"], ["star-3-5"]).values()}, {"star-3-5@sb0001"})
        picked = self.select(None, None, "SELECT DISTINCT cell FROM pick(['star-3-5', 'org-star-7-5@sb0002'], [])")
        self.assertEqual({r["cell"] for r in picked},
                         {"star-3-5@sa0001", "star-3-5@sb0001", "star-3-5", "org-star-7-5@sb0002"})

    def test_a_key_counts_once_a_cell(self):
        got = self.select(None, None, "SELECT cell, k, count(*) n FROM runs GROUP BY ALL")
        self.assertEqual({(r["cell"], r["k"]): r["n"] for r in got if r["k"] in ("k2", "kr")},
                         {("star-3-5@sa0001", "k2"): 1, ("halve-7-10@sa0001", "kr"): 1,
                          ("halve-7-10@sb0001", "kr"): 1})


if __name__ == "__main__":
    unittest.main()
