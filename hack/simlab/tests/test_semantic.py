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
               lines (a retired name, older runs', keeps its own), every
               entry or table no sim writes names its lines, its writers
               or the sims whose older runs hold it, and a writer's names
               (only) start with _
  Populations  the population rule over the rows each sim writes: a
               quantity split into _honest and _dishonest declares its
               aggregate, a count or sum is written as its parts alone and
               any other kind beside its unsuffixed all-peer value (of the
               same kind), a _dishonest part beside its _honest one, which
               every sim that writes the quantity writes, the population
               last; an aggregate only on a split quantity
  Retired      the spec's rules of retired, replaced_by and rename: each
               kind of break is found
  FarmNames    the row fields the farm reads are declared rows: worker.py's
               THIN, those store_host.py reads of a row, farm.py's report
               defaults, and every test matrix's [report] and [[derive]]
               fields (farm.py's Matrix.row_fields); the farm's code is
               read, never changed; the controller reads a renamed field
               by its current name (row_fields keep its older names too,
               current_row and derived_env read them under it)

  python3 hack/simlab/tests/test_semantic.py
"""

import copy
import importlib.util
import re
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

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
        cases.append((d, "[[scored]] lag_p50_ms: r.no_such_field isn't one of fields.toml's current rows"))
        # A retired name: older runs hold it, and the rows view reads a
        # renamed one under its replacement's name, which a metric reads.
        d = copy.deepcopy(doc)
        d["scored"][0]["sql"] = "r.startup_p99_ms"
        cases.append((d, "[[scored]] lag_p50_ms: r.startup_p99_ms isn't one of fields.toml's current rows "
                         "(retired, with no replacement)"))
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
        whos = [*e["sims"], *e.get("lines", ()), *e.get("writers", ())]
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

    def test_what_no_sim_writes_names_its_lines_or_writers(self):
        doc = vocab.fields()
        for where, key, e in [*self.entries(), *((f"tables.{t}", t, tab) for t, tab in doc["tables"].items())]:
            if not e["sims"]:
                with self.subTest(where):
                    self.assertTrue(e.get("lines") or e.get("writers") or e.get("retired"))
            # A writer's name is apart from a sim's.
            with self.subTest(where, underscore=True):
                self.assertEqual(key.startswith("_"), bool(e.get("writers")))


class Populations(unittest.TestCase):
    POP = re.compile(r"_(honest|dishonest)$")

    def written(self):
        """{sim: {name: its entry's aggregate}} of the rows each sim writes."""
        from simlab import spec
        out = {s: {} for s in spec.SIMS}
        for key, e in vocab.fields()["rows"].items():
            for s in e["sims"]:
                for n in vocab.expand(key, e, s):
                    out[s][n] = e.get("aggregate")
        return out

    def test_a_split_quantity_is_its_parts_or_beside_its_whole(self):
        from simlab import spec
        for s, names in self.written().items():
            for n, agg in names.items():
                m = self.POP.search(n)
                if not m:
                    continue
                whole = n[:m.start()]
                with self.subTest(sim=s, name=n):
                    self.assertIn(agg, spec.AGGREGATES, "a split quantity declares its aggregate")
                    if agg in spec.ADDITIVE:
                        self.assertNotIn(whole, names, "a count or sum is written as its parts alone")
                    else:
                        self.assertIn(whole, names, "the all-peer value its parts can't make is written")
                        self.assertEqual(names[whole], agg)
                    if m.group(1) == "dishonest":
                        self.assertIn(f"{whole}_honest", names)

    def test_every_sim_that_writes_a_split_quantity_writes_its_honest_part(self):
        w = self.written()
        split = {n[:-len("_honest")] for names in w.values() for n in names if n.endswith("_honest")}
        for s, names in w.items():
            for whole in sorted(split):
                if whole in names or f"{whole}_dishonest" in names:
                    with self.subTest(sim=s, name=whole):
                        self.assertIn(f"{whole}_honest", names)

    def test_the_population_comes_last(self):
        for s, names in self.written().items():
            for n in names:
                with self.subTest(sim=s, name=n):
                    self.assertNotRegex(n, r"_(?:honest|dishonest)_")

    def test_an_aggregate_is_a_split_quantitys(self):
        for s, names in self.written().items():
            for n, agg in names.items():
                if agg is not None:
                    with self.subTest(sim=s, name=n):
                        self.assertTrue(self.POP.search(n) or f"{n}_honest" in names, "no population split")


class Retired(unittest.TestCase):
    """fields.toml's retired names, as spec.check holds them: no sim, line
    or writer writes one; a replacement only on a retired name, by sim only
    for the sims it's retired for, a name of its section those sims write,
    never a family's; a rename, a rows field's, with its replacement."""

    def test_each_break_is_found(self):
        from simlab import spec
        doc = vocab.fields()
        self.assertEqual(spec.check(doc), [])
        base = {"type": "int", "description": "x"}
        cases = [
            ("rows", "x", {**base, "sims": ["ppspp"], "retired": ["strims-sim"]},
             "[rows.x]: retired, and written by a sim, a line or a writer"),
            ("rows", "x", {**base, "sims": [], "writers": ["suite"], "retired": ["ppspp"]},
             "[rows.x]: retired, and written by a sim, a line or a writer"),
            ("rows", "x", {**base, "sims": ["ppspp"], "replaced_by": "lag_p99_ms"},
             "[rows.x]: replaced_by on a name that isn't retired"),
            ("rows", "x", {**base, "sims": [], "retired": ["ppspp"], "replaced_by": {"strims-sim": "lag_p99_ms"}},
             "[rows.x]: replaced_by for ['strims-sim'], which it isn't retired for"),
            ("rows", "x", {**base, "sims": [], "retired": ["ppspp"], "rename": True},
             "[rows.x]: rename without replaced_by, or off the rows"),
            ("rows", "x", {**base, "sims": [], "retired": ["strims-sim"], "replaced_by": "worst_region_p99_ms"},
             "[rows.x]: replaced_by worst_region_p99_ms, which strims-sim doesn't write"),
            ("rows", "x", {**base, "sims": [], "retired": ["ppspp"], "replaced_by": "startup_p99_ms"},
             "[rows.x]: replaced_by startup_p99_ms, which ppspp doesn't write"),
            ("rows", "x_{q}", {**base, "sims": [], "retired": ["ppspp"], "params": {"q": ["a"]},
                               "replaced_by": "lag_p99_ms"},
             "[rows.x_{q}]: replaced_by on a family"),
            ("seed_decisions", "x", {**base, "sims": [], "retired": ["ppspp"], "replaced_by": "rate_bps",
                                     "rename": True},
             "[columns.seed_decisions.x]: rename without replaced_by, or off the rows"),
            ("seed_decisions", "x", {**base, "sims": [], "retired": ["ppspp"], "replaced_by": "lag_p99_ms"},
             "[columns.seed_decisions.x]: replaced_by lag_p99_ms, which ppspp doesn't write"),
        ]
        for where, key, e, want in cases:
            d = copy.deepcopy(doc)
            (d["rows"] if where == "rows" else d["columns"][where])[key] = e
            with self.subTest(want):
                got = spec.check(d)
                self.assertIn(want, got)
        # A rename by sim, each sim's replacement one it writes, is sound.
        d = copy.deepcopy(doc)
        d["rows"]["x"] = {**base, "sims": [], "retired": ["strims-sim", "ppspp"], "rename": True,
                          "replaced_by": {"strims-sim": "lag_p99_ms", "ppspp": "worst_region_p99_ms"}}
        self.assertEqual(spec.check(d), [])


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

    def test_the_controller_reads_a_renamed_field_by_its_current_name(self):
        farm = farm_module("farm")
        doc = copy.deepcopy(vocab.fields())
        doc["rows"]["lag_old_ms"] = {"sims": [], "retired": ["ppspp"], "replaced_by": "lag_p99_ms", "rename": True,
                                     "type": "int", "unit": "ms", "description": "x"}
        with tempfile.TemporaryDirectory() as d, mock.patch.object(vocab, "fields", lambda path=None: doc):
            path = Path(d) / "m.toml"
            path.write_text('package = "strims-ppspp"\ntarget = "sim"\ntest = "sim_job"\n[arms]\na = "HEAD"\n'
                            '[report]\ncolumns = ["lag_p99_ms:median", "delivery:mean"]\n')
            m = farm.Matrix(str(path))
            self.assertEqual(m.sim, "ppspp")
            # An older binary's row holds the retired name: the index line keeps it.
            self.assertEqual(m.row_fields(), ["lag_p99_ms", "lag_old_ms", "delivery"])
            self.assertEqual(farm.current_row("ppspp", {"lag_old_ms": 900, "delivery": 1.0}),
                             {"lag_p99_ms": 900, "delivery": 1.0})
            # A testbed's row, no sim's, reads as written.
            self.assertEqual(farm.current_row(None, {"lag_old_ms": 900}), {"lag_old_ms": 900})
            job = farm.Job(id="j", arm="a", commit="c", source="s", bin="b", test="t", env=(),
                           derive=(("SIM_X", "lag_old_ms:round"), ("SIM_Y", "lag_p99_ms")))
            for row in ({"lag_old_ms": 899.6}, {"lag_p99_ms": 899.6}):
                with self.subTest(row=row):
                    self.assertEqual(farm.derived_env(job, row, "ppspp"), ({"SIM_X": "900", "SIM_Y": "899.6"}, None))


if __name__ == "__main__":
    unittest.main()
