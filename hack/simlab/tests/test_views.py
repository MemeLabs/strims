#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.views: hack/console/views' views read and checked, every
`$name` of their SQL a param; a view saved and read back, its files as
the UI's Workbench reads them; bad names and views refused with nothing
written; views read from disk at each call; a view run over a catalog of
its own, its params' defaults and numbers as the UI gives them:

  python3 hack/simlab/tests/test_views.py
"""

import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

from simlab import engine, views  # noqa: E402
from simlab.views import Param, View  # noqa: E402


def sql_params(sql):
    """The `$name`s a statement takes outside its strings and comments, as
    the UI's Workbench finds them (paramNames)."""
    bare = re.sub(r"--[^\n]*|/\*[\s\S]*?\*/|'(?:[^']|'')*'", " ", sql)
    return set(re.findall(r"\$([A-Za-z_]\w*)", bare))


class Saved(unittest.TestCase):
    def test_the_consoles_views_check(self):
        got = views.saved()
        self.assertEqual([v.name for v in got], sorted(p.stem for p in views.VIEWS.glob("*.sql")))
        self.assertGreaterEqual(len(got), 11)
        for v in got:
            with self.subTest(view=v.name):
                self.assertEqual(views.check(v.name, v), v)
                self.assertLessEqual(sql_params(v.sql), {p.name for p in v.params})
                self.assertEqual(views.get(v.name), v)
                j = v.json()
                self.assertEqual(set(j) - {"todo"}, {"name", "title", "description", "params", "sql", "spec"})
                self.assertEqual(j["params"], json.loads((views.VIEWS / f"{v.name}.json").read_text())["params"])


class Files(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-views-")
        self.addCleanup(tmp.cleanup)
        self.root = Path(tmp.name) / "views"

    def files(self):
        return sorted(p.name for p in self.root.iterdir()) if self.root.exists() else []

    def test_a_saved_view_comes_back(self):
        v = views.save("my-view", View("ignored", "Mine", "", [{"name": "exp", "kind": "exp"}],
                                       "SELECT $exp AS e", {"mark": "bar"}), self.root)
        self.assertEqual(v, View("my-view", "Mine", "", [Param("exp", "exp")], "SELECT $exp AS e\n",
                                 {"mark": "bar"}))
        self.assertEqual([x.json() for x in views.saved(self.root)], [{
            "name": "my-view", "title": "Mine", "description": "", "params": [{"name": "exp", "kind": "exp"}],
            "sql": "SELECT $exp AS e\n", "spec": {"mark": "bar"}}])
        # The files: the statement with a newline, JSON at indent 2.
        self.assertEqual(self.files(), ["my-view.json", "my-view.sql", "my-view.vl.json"])
        self.assertEqual((self.root / "my-view.json").read_text(),
                         '{\n  "title": "Mine",\n  "description": "",\n  "params": [\n    {\n      "name": "exp",\n'
                         '      "kind": "exp"\n    }\n  ]\n}\n')
        self.assertEqual((self.root / "my-view.vl.json").read_text(), '{\n  "mark": "bar"\n}\n')

    def test_a_view_saved_again_takes_its_new_files(self):
        params = [Param("delay", "number", 1250, "playout delay"), Param("align", "align", "start"),
                  Param("t", "text"), Param("which", "choice", "a", options=["a", "b"])]
        views.save("v", View("v", None, None, params, "SELECT 1\n", {"mark": "bar"}, "a note"), self.root)
        v = views.get("v", self.root)
        self.assertEqual((v.title, v.description, v.params, v.todo), ("v", "", params, "a note"))
        self.assertEqual(v.json()["params"][0], {"name": "delay", "kind": "number", "default": 1250,
                                                 "label": "playout delay"})
        views.save("v", View("v", "No chart", "", [], "SELECT 2", None), self.root)
        v = views.get("v", self.root)
        self.assertEqual((v.title, v.sql, v.spec, v.todo, v.params), ("No chart", "SELECT 2\n", None, None, []))
        self.assertNotIn("v.vl.json", self.files())
        self.assertNotIn("todo", v.json())

    def test_bad_names_and_views_are_refused(self):
        good = View("x", "t", "", [], "SELECT 1", None)
        for name in ("../escape", "..%2Fescape", "My", "", "1a", "a.b", "a/b", "a" * 65, None):
            with self.subTest(name=name), self.assertRaises(ValueError):
                views.save(name, good, self.root)
        self.assertIsNotNone(views.save("a" * 64, good, self.root))
        bad = [
            View("x", "t", "", [], "  \n", None),
            View("x", "t", "", [], None, None),
            View("x", 3, "", [], "SELECT 1", None),
            View("x", "t", "", [], "SELECT 1", [1]),
            View("x", "t", "", {"exp": "exp"}, "SELECT 1", None),
            View("x", "t", "", [{"name": "exp", "kind": "experiment"}], "SELECT 1", None),
            View("x", "t", "", [{"name": "$exp", "kind": "exp"}], "SELECT 1", None),
            View("x", "t", "", [{"name": "exp"}], "SELECT 1", None),
            View("x", "t", "", [{"name": "exp", "kind": "exp", "picker": "x"}], "SELECT 1", None),
            View("x", "t", "", [Param("n", "number", {"a": 1})], "SELECT 1", None),
            View("x", "t", "", [Param("n", "number", float("nan"))], "SELECT 1", None),
            View("x", "t", "", [Param("n", "choice", options="ab")], "SELECT 1", None),
            View("x", "t", "", [Param("n", "number"), Param("n", "seed")], "SELECT 1", None),
            {"sql": "SELECT 1"},
        ]
        for v in bad:
            with self.subTest(view=v), self.assertRaises(ValueError):
                views.save("bad", v, self.root)
        self.assertNotIn("bad.sql", self.files())

    def test_views_are_read_from_disk_at_each_call(self):
        self.assertEqual(views.saved(self.root), [])
        self.root.mkdir()
        (self.root / "hand.sql").write_text("SELECT 1\n")
        self.assertEqual(views.saved(self.root), [View("hand", "hand", "", [], "SELECT 1\n", None)])
        (self.root / "hand.json").write_text('{"title": "By hand", "params": [{"name": "seed", "kind": "seed"}]}')
        (self.root / "hand.vl.json").write_text('{"mark": "line"}')
        v = views.get("hand", self.root)
        self.assertEqual((v.title, v.params, v.spec), ("By hand", [Param("seed", "seed")], {"mark": "line"}))
        # Files of other names are no views.
        for other in ("Upper.sql", "a.b.sql", "notes.txt"):
            (self.root / other).write_text("SELECT 1\n")
        self.assertEqual([v.name for v in views.saved(self.root)], ["hand"])
        for name in ("Upper", "a.b", "../views/hand", None):
            with self.assertRaises(KeyError):
                views.get(name, self.root)
        (self.root / "hand.sql").unlink()
        self.assertEqual(views.saved(self.root), [])
        with self.assertRaises(KeyError):
            views.get("hand", self.root)

    def test_a_broken_file_says_which(self):
        self.root.mkdir()
        (self.root / "b.sql").write_text("SELECT 1\n")
        for text in ("{", '{"params": {"exp": "exp"}}', "[]"):
            (self.root / "b.json").write_text(text)
            with self.assertRaisesRegex(ValueError, re.escape(str(self.root / "b.json"))):
                views.saved(self.root)


@unittest.skipIf(engine.duckdb is None, "needs the duckdb module (uv)")
class Run(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-views-run-")
        cls.dir = Path(cls.tmp.name)
        lake = cls.dir / "lake"
        (lake / "data").mkdir(parents=True)
        w = engine.writer(lake)
        try:
            w.execute("CREATE TABLE runs AS SELECT 'e1' AS exp_id, range AS seed FROM range(2001, 2011)")
        finally:
            w.close()
        cls.pool = engine.Pool(lake, cls.dir / "spill", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        cls.root = cls.dir / "views"
        views.save("seeds", View("seeds", "Seeds", "", [
            Param("exp", "exp"), Param("first", "seed"), Param("n", "number", 3), Param("tag", "search", ""),
        ], "SELECT seed, $tag AS tag FROM runs WHERE exp_id = $exp AND seed >= $first ORDER BY seed LIMIT $n",
            None), cls.root)
        views.save("copy", View("copy", "", "", [], "COPY runs TO 'x.csv'", None), cls.root)

    @classmethod
    def tearDownClass(cls):
        cls.pool.close()
        cls.tmp.cleanup()

    def run_view(self, name, params=None, **kw):
        return views.run(self.pool, name, params, root=self.root, **kw)

    def status(self, name, params=None):
        try:
            self.run_view(name, params)
        except engine.QueryError as e:
            return e.status, str(e)
        return 200, ""

    def test_defaults_fill_the_params_not_given(self):
        r = self.run_view("seeds", {"exp": "e1", "first": 2004})
        self.assertEqual(r.values(), [[2004, ""], [2005, ""], [2006, ""]])
        self.assertEqual(self.run_view("seeds", {"exp": "e1", "first": 2004, "n": "", "tag": None}).values(),
                         r.values())
        r = self.run_view("seeds", {"exp": "e1", "first": 2009, "n": 5, "tag": "x", "other": 1})
        self.assertEqual(r.values(), [[2009, "x"], [2010, "x"]])

    def test_a_seeds_and_a_numbers_string_is_its_number(self):
        r = self.run_view("seeds", {"exp": "e1", "first": "2008", "n": "2.0"})
        self.assertEqual(r.values(), [[2008, ""], [2009, ""]])
        self.assertEqual(self.status("seeds", {"exp": "e1", "first": "x"}), (400, "$first is a number, not 'x'"))

    def test_a_param_missing_fails(self):
        self.assertEqual(self.status("seeds", {"first": 2001}), (400, "no value for $exp"))
        self.assertEqual(self.status("seeds", {"exp": "e1", "first": ""}), (400, "no value for $first"))

    def test_the_gate_and_the_cap_hold(self):
        self.assertEqual(self.status("copy")[0], 403)
        r = self.run_view("seeds", {"exp": "e1", "first": 2001, "n": 10}, max_rows=4)
        self.assertEqual((len(r.rows), r.truncated), (4, True))
        with self.assertRaises(KeyError):
            self.run_view("none")


if __name__ == "__main__":
    unittest.main()
