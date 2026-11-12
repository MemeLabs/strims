#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.semantic: crates/strims-wan/metrics.toml and the naming rules of
crates/strims-wan/fields.toml.

  Scored       suite.py's METRICS, GUARDRAILS (in their order), SHOWN and
               HOST_METRICS are metrics.toml's; a share and a per_useful
               compile to their SQL
  Problems     the document checks, every field a metric reads declared,
               the run metrics verdict.run_metrics', every macro the lab's,
               every dimension's values a section; each kind of break is
               found
  Restated     the lab's SQL restates no lake metric; a restatement
               outside the macro's definition is found, one inside it isn't
  Naming       fields.toml's names: a unit suffix agrees with the declared
               unit (and names that all carry one declare it), percentiles
               are _pNN, families hold names for each of their sims and
               lines, every entry or table no sim writes names its lines
  FarmNames    the row fields the farm reads are declared rows: worker.py's
               THIN, those store_host.py reads of a row, farm.py's report
               defaults, and every test matrix's [report] and [[derive]]
               fields (farm.py's Matrix.row_fields); the farm's code is
               read, never changed

  python3 hack/simlab/tests/test_semantic.py
"""

import copy
import importlib.util
import re
import sys
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import semantic, suite, vocab  # noqa: E402

# A name's unit by its last unit part (before a percentile, a population
# or a censoring suffix): the naming rule.
SUFFIX_UNITS = {"ms": "ms", "us": "us", "s": "s", "secs": "s", "bps": "bytes/s", "bytes": "bytes", "kib": "KiB",
                "mb": "MB", "mbit": "Mbit/s", "gib": "GiB"}
TRAILING = re.compile(r"p\d+|honest|hostile|dishonest|censored|startup|steady")


def suffix_unit(name):
    """The unit a name's suffix says, or None."""
    parts = name.split("_")
    while parts and TRAILING.fullmatch(parts[-1]):
        parts.pop()
    if parts[-2:] == ["ms", "s"]:
        return "ms x s"
    return SUFFIX_UNITS.get(parts[-1]) if parts else None


class Scored(unittest.TestCase):
    def test_suites_metrics_are_metrics_tomls(self):
        self.assertEqual(repr(semantic.scored()), repr(suite.METRICS))
        self.assertEqual(list(semantic.guardrails().items()), list(suite.GUARDRAILS.items()))
        self.assertEqual(semantic.shown(), suite.SHOWN)
        self.assertEqual(semantic.host_metrics(), suite.HOST_METRICS)

    def test_a_share_and_a_per_useful_compile_to_their_sql(self):
        self.assertEqual(semantic.share("r.a", "r.b"), "CASE WHEN r.b <> 0 THEN coalesce(r.a, 0) / (r.b) END")
        self.assertEqual(semantic.per_useful("x", "y"),
                         "CASE WHEN r.useful_bytes <> 0 THEN (r.x + r.y) / r.useful_bytes END")
        doc = semantic.load()
        kinds = {k for m in doc["scored"] for k in ("sql", "share", "per_useful") if k in m}
        self.assertEqual(kinds, {"sql", "share", "per_useful"})


class Problems(unittest.TestCase):
    def test_the_document_checks(self):
        self.assertEqual(semantic.problems(), [])
        self.assertEqual(semantic.restatements(), [])

    def test_each_break_is_found(self):
        doc = semantic.load()
        cases = []
        d = copy.deepcopy(doc)
        d["scored"][0]["sql"] = "r.no_such_field"
        cases.append((d, "[[scored]] lag_p50_ms: r.no_such_field isn't one of fields.toml's rows"))
        d = copy.deepcopy(doc)
        del d["run_metrics"]["osc_acf"]
        cases.append((d, "[run_metrics]: "))
        d = copy.deepcopy(doc)
        d["lake"]["playback"]["macro"] = "no_such_macro"
        cases.append((d, "[lake.playback]: macro no_such_macro isn't in hack/simlab/sql"))
        d = copy.deepcopy(doc)
        d["dimensions"]["phase"]["values"] = "vocab:nope"
        cases.append((d, "[dimensions.phase]: values vocab:nope: no vocabulary section nope"))
        d = copy.deepcopy(doc)
        d["guardrails"] = d["guardrails"][1:]
        cases.append((d, "guardrails "))
        d = copy.deepcopy(doc)
        d["scored"][1]["share"] = {"num": "r.a", "den": "r.b"}
        cases.append((d, "lag_p99_ms: one of sql, share and per_useful"))
        for d, want in cases:
            with self.subTest(want=want):
                got = semantic.problems(d)
                self.assertTrue(any(s.startswith(want) or want in s for s in got), got)


class Restated(unittest.TestCase):
    def test_a_restatement_outside_its_macro_is_found(self):
        with tempfile.TemporaryDirectory() as d:
            view = Path(d) / "view.sql"
            view.write_text("SELECT coalesce(p.phase, 'steady') AS phase\nFROM x p;\n")
            own = Path(d) / "macros.sql"
            own.write_text("CREATE OR REPLACE MACRO viewer_phases_of(runs) AS TABLE\n"
                           "    SELECT coalesce(p.phase, 'steady') FROM x p;\n")
            got = semantic.restatements([view, own])
        self.assertEqual(len(got), 1, got)
        self.assertRegex(got[0], r"view\.sql:1: .* restates viewer_phases \(viewer_phases_of\)$")


class Naming(unittest.TestCase):
    def entries(self):
        doc = vocab.fields()
        out = [(f"rows.{k}", k, e) for k, e in doc["rows"].items()]
        out += [(f"meta.{k}", k, e) for k, e in doc["meta"].items()]
        out += [(f"columns.{t}.{k}", k, e) for t, cols in doc["columns"].items() for k, e in cols.items()]
        return out

    def names(self, key, e):
        whos = [*e["sims"], *e.get("lines", ())]
        return {n for who in whos for n in vocab.expand(key, e, who)}

    def test_a_unit_suffix_agrees_with_the_declared_unit(self):
        for where, key, e in self.entries():
            names = self.names(key, e)
            units = {suffix_unit(n) for n in names}
            with self.subTest(where):
                if e.get("unit"):
                    self.assertEqual(sorted(n for n in names if suffix_unit(n) not in (None, e["unit"])), [],
                                     f"declared {e['unit']}")
                elif len(units) == 1:
                    self.assertIsNone(units.pop(), "every name says its unit: declare it")

    def test_percentiles_are_pnn(self):
        for where, key, e in self.entries():
            for n in self.names(key, e):
                with self.subTest(n=n, where=where):
                    for p in re.findall(r"(?:^|_)(p\d+)(?=_|$)", n):
                        self.assertRegex(p, r"^p\d\d$")
                    self.assertNotRegex(n, r"(?:^|_)(?:median|q\d+|pct\d*)(?:_|$)")

    def test_families_hold_names_for_each_of_their_sims_and_lines(self):
        for where, key, e in self.entries():
            if "{" not in key:
                continue
            for who in [*e["sims"], *e.get("lines", ())]:
                with self.subTest(where=where, who=who):
                    names = vocab.expand(key, e, who)
                    self.assertTrue(names)
                    self.assertTrue(all("{" not in n for n in names), names)

    def test_what_no_sim_writes_names_its_lines(self):
        doc = vocab.fields()
        for where, _, e in [*self.entries(), *((f"tables.{t}", t, tab) for t, tab in doc["tables"].items())]:
            if not e["sims"]:
                with self.subTest(where):
                    self.assertTrue(e.get("lines"))


FARM = HACK / "sim-farm"


def farm_module(name):
    """hack/sim-farm's `name`.py, loaded as a module of its own (farm.py
    imports worker beside it)."""
    if str(FARM) not in sys.path:
        sys.path.insert(0, str(FARM))
    spec = importlib.util.spec_from_file_location(f"sim_farm_{name}", FARM / f"{name}.py")
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


class FarmNames(unittest.TestCase):
    """The names the farm reads of a job's row are declared, so a rename
    (Phase 2's) knows every reader. The farm's hosts run worker.py and
    store_host.py, and the controller farm.py: they never rename one."""

    @classmethod
    def setUpClass(cls):
        cls.rows = set(vocab.declared()["rows"])

    def test_the_hosts_names(self):
        worker = farm_module("worker")
        self.assertEqual(sorted(set(worker.THIN) - self.rows), [], "worker.THIN")
        store_host = (HACK / "simlab" / "store_host.py").read_text()
        read = set(re.findall(r'\brow\.get\("([a-z_][a-z0-9_]*)"\)', store_host))
        self.assertTrue({"trace", "frames", "bytes"} <= read, read)
        # But `series`, the JSON an older binary's row carried, which the
        # fold refuses (no sim writes it now).
        self.assertEqual(sorted(read - self.rows), ["series"], "store_host's row.get")

    def test_the_controllers_names(self):
        farm = farm_module("farm")
        # A test matrix with no [report] reports these (farm.py's Matrix).
        defaults = ["delivery", "lag_p50_ms", "lag_p99_ms"]
        self.assertIn('columns = ["delivery:mean,min", "lag_p50_ms:median", "lag_p99_ms:median,max"]',
                      (FARM / "farm.py").read_text())
        self.assertEqual(sorted(set(defaults) - self.rows), [])
        matrices = sorted((FARM / "matrices").glob("*.toml"))
        self.assertTrue(matrices)
        tests = 0
        for path in matrices:
            m = farm.Matrix(str(path))
            # A testbed matrix's rows are the testbed harness's, not a sim's.
            if m.kind != "test":
                continue
            tests += 1
            with self.subTest(matrix=path.name):
                self.assertEqual(sorted(set(m.row_fields()) - self.rows), [])
        self.assertGreater(tests, 30)


if __name__ == "__main__":
    unittest.main()
