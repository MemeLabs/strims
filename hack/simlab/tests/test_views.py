#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.views: hack/console/views' views read and checked, every
`$name` of their SQL a param; a view saved and read back, its files as
the UI's Workbench reads them; bad names and views refused with nothing
written; views read from disk at each call; a view run over a catalog of
its own, its params' defaults and numbers as the UI gives them; and the
console's addresses:

  Addresses   testdata/links.json's addresses made by link and read by
              parse; the page's selection keys and defaults
              (selection.ts) and the workbench's marks as views.py's;
              what a link leaves out, its encoding, its numbers, its
              host; what it refuses
  Filters     every saved spec's filter transforms translate to SQL, and
              keep the rows JavaScript keeps; the forms that don't say why
  Graphs      every saved view's panels, time fields, encoded columns and
              legends, the vocabulary's colours among them; vocabulary
              references expanded as domains.ts expands them
  Selections  over golden_cli's fixture catalog (its bins given the
              columns run_timeline reads): a moment and ranges on
              run_timeline, a range and a viewer on its stall track, a
              collapse_grid cell, a workbench chart; check's problems: a
              bad experiment, cell or seed, a selection matching no row

  python3 hack/simlab/tests/test_views.py
"""

import json
import os
import re
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
sys.path.insert(0, str(HACK))
sys.path.insert(0, str(HERE))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import simlab  # noqa: E402
from simlab import catalog, engine, store_host, views, vocab  # noqa: E402
from simlab.views import Param, View  # noqa: E402

TESTDATA = HERE / "testdata"
LINKS = json.loads((TESTDATA / "links.json").read_text())
CONSOLE_TS = HACK.parent / "src" / "devtools" / "components" / "console"
STALLS = "Stalls at the playout delay picked, as long as picked or longer"
LAG = "Lag across viewers (playout delays dashed)"
VERDICT = "Verdict: phases, unhealthy seconds, and the episodes"
# golden_cli's jobs write bins of the meter, the uplink's bytes and the
# timeouts alone; the sims' bins today also hold what run_timeline's seed,
# copies, bytes, data and packets tracks read.
BINS = """
CREATE OR REPLACE VIEW bins AS
SELECT x.k, x.src, x.bin, x.t_ms, nullif(x.seed_meter, -1) AS seed_meter, nullif(x.seed_up_bytes, -1) AS seed_up_bytes,
       nullif(x.viewer_timeouts, -1) AS viewer_timeouts, nullif(x.seed_meter, -1) AS capacity,
       nullif(x.seed_up_bytes, -1) * 10 AS avail, 2 AS copies, 100 AS seed_have_bytes, 4096 AS viewer_data_bytes,
       3 AS seed_up_packets, 5 AS viewer_up_packets
FROM raw.bins x JOIN main.jobs j_ ON j_.k = x.k AND j_.src = x.src
"""


def catalog_fixture(root):
    """golden_cli's fixture under `root`, indexed, its bins given the
    columns run_timeline reads (BINS): the catalog's lake."""
    import golden_cli

    saved = os.environ.get("SIM_DB_ROOT")
    try:
        golden_cli.build(root)
        with mock.patch.dict(os.environ, {"TZ": "UTC"}), \
                mock.patch.object(simlab.simdb(), "main_checkout", lambda: root), \
                mock.patch.object(store_host, "main_checkout", lambda: root):
            catalog.index(0, None)
    finally:
        os.environ.pop("SIM_DB_ROOT", None)
        if saved is not None:
            os.environ["SIM_DB_ROOT"] = saved
    lake = root / "target" / "sim-db" / "lake"
    w = engine.writer(lake)
    try:
        w.execute(BINS)
    finally:
        w.close()
    return lake


def sql_params(sql):
    """The `$name`s a statement takes outside its strings and comments, as
    the UI's Workbench finds them (paramNames)."""
    bare = re.sub(r"--[^\n]*|/\*[\s\S]*?\*/|'(?:[^']|'')*'", " ", sql)
    return set(re.findall(r"\$([A-Za-z_]\w*)", bare))


def _results(e):
    """The text constants expression `e` (a node of DuckDB's parse tree)
    can be: itself, a CASE's results, COALESCE's arguments, a cast's
    operand; nothing for any other expression."""
    if not isinstance(e, dict):
        return set()
    if e.get("class") == "CONSTANT":
        v = e["value"]
        return {v["value"]} if not v["is_null"] and v["type"]["id"] == "VARCHAR" else set()
    if e.get("class") == "CASE":
        return set().union(*(_results(c["then_expr"]) for c in e["case_checks"]), _results(e["else_expr"]))
    if e.get("type") == "OPERATOR_COALESCE" or (e.get("class") == "FUNCTION"
                                                 and e.get("function_name") in ("coalesce", "ifnull")):
        return set().union(*(_results(c) for c in e["children"]))
    if e.get("class") == "CAST":
        return _results(e["child"])
    return set()


def _columns(node):
    """Query node `node`'s output columns as [(name or None, [expression])]:
    a SELECT's list, a set operation's left side's names over both sides'
    expressions by position."""
    if node.get("type") == "SELECT_NODE":
        out = []
        for e in node["select_list"]:
            name = e.get("alias") or (e["column_names"][-1] if e.get("class") == "COLUMN_REF" else None)
            out.append((name, [e]))
        return out
    if node.get("type") == "SET_OPERATION_NODE":
        left, right = _columns(node["left"]), _columns(node["right"])
        return [(name, es + (right[i][1] if i < len(right) else [])) for i, (name, es) in enumerate(left)]
    return []


def returned(sql, names):
    """{name: {text}}: the text constants a column called one of `names`
    can hold, in any of statement `sql`'s queries (its CTEs', subqueries'
    and set operations'), from DuckDB's parse tree; a column's values from
    a table aren't known, and count as none."""
    con = engine.duckdb.connect()
    try:
        tree = json.loads(con.execute("SELECT json_serialize_sql(?)", [sql]).fetchone()[0])
    finally:
        con.close()
    if tree.get("error"):
        raise ValueError(tree.get("error_message"))
    out = {n: set() for n in names}

    def walk(v, named=True):
        # A set operation's sides take its names, not their own.
        if isinstance(v, dict):
            for name, es in _columns(v) if named else ():
                if name in out:
                    for e in es:
                        out[name] |= _results(e)
            side = v.get("type") == "SET_OPERATION_NODE"
            for k, x in v.items():
                walk(x, not (side and k in ("left", "right")))
        elif isinstance(v, list):
            for x in v:
                walk(x)

    walk(tree)
    return out


class Saved(unittest.TestCase):
    def test_the_consoles_views_check(self):
        got = views.saved()
        self.assertEqual([v.name for v in got], sorted(p.stem for p in views.VIEWS.glob("*.sql")))
        self.assertGreaterEqual(len(got), 11)
        for v in got:
            with self.subTest(view=v.name):
                self.assertEqual(views.checked(v.name, v), v)
                self.assertLessEqual(sql_params(v.sql), {p.name for p in v.params})
                self.assertEqual(views.get(v.name), v)
                j = v.json()
                self.assertEqual(set(j) - {"todo"}, {"name", "title", "description", "params", "sql", "spec"})
                self.assertEqual(j["params"], json.loads((views.VIEWS / f"{v.name}.json").read_text())["params"])

    @unittest.skipIf(engine.duckdb is None, "needs the duckdb module (uv)")
    def test_the_phases_and_reasons_a_view_makes_are_the_documents(self):
        doc = {"phase": set(vocab.phases()), "reason": set(vocab.reasons())}
        made = set()
        for v in views.saved():
            with self.subTest(view=v.name):
                for col, got in returned(v.sql, doc).items():
                    self.assertLessEqual(got, doc[col], f"{v.name}'s {col}s")
                    made |= got
        self.assertIn("steady", made, "a view's unknown phase falls back to steady")

    @unittest.skipIf(engine.duckdb is None, "needs the duckdb module (uv)")
    def test_the_values_a_column_can_hold(self):
        sql = """
            WITH s AS (
                SELECT CASE WHEN x THEN 'departure' ELSE coalesce(p.phase, 'steady') END AS phase,
                       p.kind = 'depart' AS gone FROM t p
            )
            SELECT 'a' AS part, phase, 'x' AS reason FROM s
            UNION ALL SELECT 'b', CAST('tail' AS VARCHAR), NULL FROM s WHERE phase IN (SELECT 'startup' AS phase)
            UNION ALL SELECT 'c', ifnull(phase, 'after_up'), upper('y') FROM s
        """
        self.assertEqual(returned(sql, ("phase", "reason", "part")), {
            "phase": {"departure", "steady", "tail", "startup", "after_up"}, "reason": {"x"},
            "part": {"a", "b", "c"}})
        # A union's columns are its first query's, however the later ones name theirs.
        sql = "SELECT 'p' AS series FROM t UNION ALL SELECT phase FROM t UNION ALL SELECT 'unhealthy' FROM t"
        self.assertEqual(returned(sql, ("phase", "series")), {"phase": set(), "series": {"p", "unhealthy"}})


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

    def test_the_pages_routes_are_no_views(self):
        for name in views.ROUTES:
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, f"{name} is the console's"):
                views.save(name, View(name, "", "", [], "SELECT 1", None), self.root)
            (self.root / f"{name}.sql").write_text("SELECT 1\n")
            self.assertNotIn(name, [v.name for v in views.saved(self.root)])
            (self.root / f"{name}.sql").unlink()


def page_defaults():
    """selection.ts's DEFAULTS literal, as [(key, value)] in its order."""
    text = (CONSOLE_TS / "selection.ts").read_text()
    body = re.search(r"const DEFAULTS: Selection = \{(.*?)\};", text, re.S)
    return re.findall(r'(\w+): "([^"]*)"', body[1])


class Addresses(unittest.TestCase):
    def test_the_links_round_trip(self):
        self.assertGreaterEqual(len(LINKS), 10)
        named = {x["name"] for x in LINKS}
        self.assertLessEqual({"run_timeline", "collapse_grid", views.WORKBENCH}, named)
        for x in LINKS:
            with self.subTest(url=x["url"]):
                self.assertEqual(views.link(x["name"], x["params"], x["sel"], base=views.BASE), x["url"])
                self.assertEqual(views.parse(x["url"]), (x["name"], x["params"], x["sel"]))
                self.assertTrue(x["url"].startswith(f"{views.BASE}{views.PAGE}{x['name']}"))

    def test_the_pages_keys_defaults_and_marks_are_views_pys(self):
        pairs = page_defaults()
        self.assertEqual(tuple(k for k, _ in pairs), views.KEYS)
        self.assertEqual(dict(pairs), views.DEFAULTS)
        wb = (CONSOLE_TS / "Workbench.tsx").read_text()
        marks = re.findall(r'"(\w+)"', re.search(r"const MARKS = \[(.*?)\];", wb, re.S)[1])
        self.assertEqual(tuple(marks), views.MARKS)

    def test_what_a_link_leaves_out_and_its_order(self):
        run = {"seed": "3007", "arm": "a", "exp": "e", "cell": "c"}
        self.assertEqual(views.link("run_timeline", {**run, "delay": "1250", "min_stall_ms": ""}, base="http://h/"),
                         "http://h/devtools.html#/console/run_timeline?exp=e&cell=c&arm=a&seed=3007")
        self.assertEqual(views.link("run_timeline", {**run, "delay": 4250}, base="http://h"),
                         "http://h/devtools.html#/console/run_timeline?exp=e&cell=c&arm=a&seed=3007&delay=4250")
        self.assertEqual(views.link("overhead", {}, base="http://h"), "http://h/devtools.html#/console/overhead")
        # A view's own param read under its kind: a seed named first is the address's seed.
        tmp = tempfile.TemporaryDirectory(prefix="simlab-views-links-")
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        views.save("firsts", View("firsts", "", "", [Param("from", "seed"), Param("exp", "exp"),
                                                    Param("n", "number", 3)], "SELECT $exp, $from, $n", None), root)
        url = views.link("firsts", {"from": "2001", "exp": "e", "n": "3"}, base="http://h", root=root)
        self.assertEqual(url, "http://h/devtools.html#/console/firsts?exp=e&seed=2001")
        self.assertEqual(views.parse(url, root), ("firsts", {"from": "2001", "exp": "e"}, None))
        with self.assertRaisesRegex(ValueError, r"firsts's n can't be set from an address"):
            views.link("firsts", {"n": "4"}, root=root)

    def test_the_query_is_url_search_params_and_sel_json_stringifys(self):
        url = views.link("experiments", {"search": "a b/c~d*e.f-g_h ü&=#"}, base="http://h")
        self.assertEqual(url, "http://h/devtools.html#/console/experiments?search=a+b%2Fc%7Ed*e.f-g_h+%C3%BC%26%3D%23")
        self.assertEqual(views.parse(url)[1], {"search": "a b/c~d*e.f-g_h ü&=#"})
        url = views.link("run_timeline", {"exp": "e"}, {"panel": "0", "at": {"t_s": [1e-7, 38.0], "series": "p99"}},
                         base="http://h")
        sel = url.split("sel=")[1]
        self.assertEqual(views.urllib.parse.unquote_plus(sel),
                         '{"panel":"Lag across viewers (playout delays dashed)","at":{"series":"p99","t_s":[1e-7,38]}}')
        for x, text in [(1.0, "1"), (0.1, "0.1"), (1e21, "1e+21"), (1e20, "100000000000000000000"), (1e-7, "1e-7"),
                        (1.5e-6, "0.0000015"), (-2.5, "-2.5"), (1 / 3, "0.3333333333333333"), (3007, "3007")]:
            self.assertEqual(views.js_number(x), text)

    def test_the_host(self):
        with mock.patch.dict(os.environ, {"SIM_CONSOLE_URL": "http://box.local:9000/"}):
            self.assertEqual(views.base(), "http://box.local:9000")
            self.assertTrue(views.link("overhead", {}).startswith("http://box.local:9000/devtools.html#/console/"))
        with mock.patch.dict(os.environ, {}, clear=False), mock.patch.object(views, "server_state", lambda: None):
            os.environ.pop("SIM_CONSOLE_URL", None)
            self.assertEqual(views.base(), views.BASE)
        state = {"pid": os.getpid(), "port": 8087, "urls": ["http://127.0.0.1:8087/", "http://192.168.1.5:8087/"]}
        with mock.patch.dict(os.environ, {}), mock.patch.object(views, "server_state", lambda: state), \
                mock.patch.object(views, "tabs", lambda server_url=None: []):
            os.environ.pop("SIM_CONSOLE_URL", None)
            self.assertEqual(views.base(), "http://192.168.1.5:8087")
            with mock.patch.object(views, "tabs", lambda server_url=None: [
                    {"id": "a", "url": "http://blinky.local:8087/devtools.html#/console/tree", "focused": True}]):
                self.assertEqual(views.base(), "http://blinky.local:8087")

    def test_panels_by_id_or_index_path(self):
        rt = views.get("run_timeline")
        for given in ("2", STALLS):
            self.assertEqual(views.selection(rt, {"panel": given, "at": {"viewer": 7}})[0],
                             {"panel": STALLS, "at": {"viewer": 7}})
        # A graph of one panel leaves it out.
        grid = views.get("collapse_grid")
        self.assertEqual(views.selection(grid, {"panel": "0", "at": {"seed": 3007, "cell": "x"}})[0],
                         {"at": {"cell": "x", "seed": 3007}})

    def test_what_a_link_refuses(self):
        run = {"exp": "e", "cell": "c", "arm": "a", "seed": "1"}
        bad = [
            ("run_timeline", {**run, "arm2": "b"}, None, r"run_timeline takes no arm2: its params are exp, cell"),
            ("run_timeline", {**run, "seed": "x"}, None, r"\$seed is a number, not 'x'"),
            ("run_timeline", run, {"panel": STALLS, "at": {"value": 3}},
             r"panel \"Stalls.*\" encodes no column 'value': its columns are t_s, t2_s, viewer"),
            # A field a transform makes is no column: the verdict track's lane.
            ("run_timeline", run, {"panel": VERDICT, "at": {"lane": "phase"}}, r"encodes no column 'lane'"),
            ("run_timeline", run, {"at": {"source": "sim"}}, r"run_timeline encodes no column 'source'"),
            ("run_timeline", run, {"panel": STALLS, "at": {"viewer": [1, 3]}},
             r"a range is on panel \"Stalls.*\"'s time field, t_s: not viewer"),
            ("run_timeline", run, {"panel": STALLS, "at": {"t_s": [46, 38]}}, r"t_s's range is \[LO, HI\]"),
            ("run_timeline", run, {"panel": STALLS, "at": {"t_s": [1, "2"]}}, r"t_s's range is \[LO, HI\]"),
            ("run_timeline", run, {"panel": "lag", "at": {"t_s": 1}}, r"run_timeline has no panel 'lag': its panels"),
            ("run_timeline", run, {"at": {}}, r"a selection's at names a field"),
            ("run_timeline", run, {"at": {"t_s": {"x": 1}}}, r"t_s's value is a string, number"),
            ("run_timeline", run, {"at": {"t_s": 1}, "where": 1}, r"a selection is"),
            ("tree", {"exp": "e"}, {"panel": "3", "at": {"stream": [0, 1]}},
             r"a range is on panel \"Depth of each node.*\"'s time field, which it has none of: not stream"),
            ("experiments", {}, {"at": {"exp_id": "e"}}, r"experiments draws no graph"),
            ("nope", {}, None, r"no view 'nope': collapse_diff, "),
            ("ask", {}, None, r"no view 'ask'"),
            (views.WORKBENCH, {"sql": "SELECT 1", "view": "tree"}, None, r"sql or view, not both"),
            (views.WORKBENCH, {"view": "nope"}, None, r"the workbench's view 'nope' is no saved view"),
            (views.WORKBENCH, {"sql": "SELECT $x AS x", "x": "t", "mark": "pie"}, None, r"marks are line, point"),
            (views.WORKBENCH, {"sql": "SELECT $sel AS s", "sel": "1"}, None, r"\$sel can't be set from an address"),
            (views.WORKBENCH, {"sql": "SELECT $exp", "cell": "c"}, None, r"the workbench takes no cell: its statement"),
            (views.WORKBENCH, {"sql": "SELECT 1", "logy": "maybe"}, None, r"logy is true or false"),
            (views.WORKBENCH, {"sql": "SELECT seed FROM t"}, {"at": {"seed": 1}}, r"workbench draws no graph"),
            (views.WORKBENCH, {"sql": "SELECT seed, t FROM t", "x": "t", "y": "seed"}, {"at": {"t": [1, 2]}},
             r"a range is on workbench's time field, which it has none of"),
        ]
        for name, params, sel, message in bad:
            with self.subTest(name=name, params=params, sel=sel), self.assertRaisesRegex(ValueError, message):
                views.link(name, params, sel, base=views.BASE)

    def test_parse_reads_what_the_page_reads(self):
        page = f"{views.BASE}{views.PAGE}"
        self.assertEqual(views.parse(f"{views.BASE}/devtools.html#/console"), ("experiments", {}, None))
        # Keys the route doesn't read are none of its params; an empty one gives the view's default.
        self.assertEqual(views.parse(f"{page}run_timeline?sql=SELECT+1&exp=e&delay=&mark=bar"),
                         ("run_timeline", {"exp": "e", "min_stall_ms": "50"}, None))
        self.assertEqual(views.parse(f"http://other:8080{views.PAGE}tree?t=45&t=50")[1], {"t": "45"})
        for url, message in [(f"{page}run_timeline?sel=%7B", r"sel isn't JSON"),
                             ("http://h/devtools.html#/other", r"not a console address"),
                             (f"{page}ask", r"the console's ask page"), (f"{page}nope", r"names no view 'nope'"),
                             (f"{page}workbench?view=nope", r"is no saved view")]:
            with self.subTest(url=url), self.assertRaisesRegex(ValueError, message):
                views.parse(url)


class Filters(unittest.TestCase):
    def test_every_saved_specs_filters_translate(self):
        n = 0
        for v in views.saved():
            for p in views.panels(v.spec):
                for f in p.filters + [f for layer in p.layers for f in layer.filters]:
                    with self.subTest(view=v.name, panel=p.id, filter=f.text):
                        self.assertIsNotNone(f.sql, f.why)
                        n += 1
        self.assertGreaterEqual(n, 25)

    def test_the_forms_that_translate(self):
        for text, sql in [
            ("datum.track === 'lag'", "\"track\" IS NOT DISTINCT FROM 'lag'"),
            ("datum.kind !== \"it's\"", "\"kind\" IS DISTINCT FROM 'it''s'"),
            ("datum['a b'] === 'x\\'y' && datum.n === 3", "(\"a b\" IS NOT DISTINCT FROM 'x''y' AND \"n\" IS NOT "
                                                         "DISTINCT FROM 3)"),
            ("!(datum.a === null || datum.b === true)", "NOT ((\"a\" IS NOT DISTINCT FROM NULL OR \"b\" IS NOT "
                                                        "DISTINCT FROM TRUE))"),
            ("datum.a === datum.b", "\"a\" IS NOT DISTINCT FROM \"b\""),
        ]:
            self.assertEqual(views.filter_sql(text), sql, text)

    def test_the_forms_that_dont_say_why(self):
        for f, why in [
            ({"field": "a", "equal": 1}, r"a predicate object"),
            ("datum.a > 3", r">: only === and !== compare here"),
            ("datum.a == 'x'", r"==: only === and !=="),
            ("datum.a", r"a field or a literal alone is no comparison"),
            ("isValid(datum.a)", r"isValid: only datum's fields and literals"),
            ("datum.a.b === 1", r"datum.a's parts or calls"),
            ("datum.a === 'x' ||", r"the expression ends early"),
            ("(datum.a === 'x'", r"a parenthesis left open"),
            ("datum.a === 1 datum.b", r"'datum' past the expression's end"),
        ]:
            with self.subTest(f=f), self.assertRaisesRegex(views.Untranslated, why):
                views.filter_sql(f)
        made = views.Rows(frozenset({"lane"}))
        with self.assertRaisesRegex(views.Untranslated, "lane is a field a transform makes"):
            views.filter_sql("datum.lane === 'phase'", made.column)
        # A panel reports the filter it can't translate.
        spec = {"transform": [{"calculate": "1", "as": "one"}],
                "vconcat": [{"transform": [{"filter": "datum.one === 1"}, {"filter": "datum.k === 'a'"}],
                             "mark": "point", "encoding": {"x": {"field": "k"}}}]}
        (p,) = views.panels(spec)
        self.assertEqual([(f.text, f.sql is None) for f in p.filters], [("datum.one === 1", True),
                                                                         ("datum.k === 'a'", False)])
        w = views.where(View("v", "", "", [], "SELECT 1", spec), {"at": {"k": "a"}})
        self.assertEqual(w.unapplied, ["datum.one === 1 (one is a field a transform makes, no column of the view's "
                                       "rows)"])

    @unittest.skipIf(engine.duckdb is None, "needs the duckdb module (uv)")
    def test_translated_filters_keep_the_rows_javascript_keeps(self):
        rows = [("lag", "p50"), ("lag", None), (None, "p99"), ("stall", "p99"), ("lag", "max")]
        con = engine.duckdb.connect()
        self.addCleanup(con.close)
        con.execute("CREATE TABLE t (i INT, track VARCHAR, series VARCHAR)")
        con.executemany("INSERT INTO t VALUES (?, ?, ?)", [(i, *r) for i, r in enumerate(rows)])
        for text, keep in [
            ("datum.track === 'lag' && datum.series !== 'p50'", lambda t, s: t == "lag" and s != "p50"),
            ("!(datum.track === 'lag')", lambda t, s: t != "lag"),
            ("datum.track === null || datum.series === 'p99'", lambda t, s: t is None or s == "p99"),
            ("datum.series !== 'p99' && !(datum.track !== 'lag')", lambda t, s: s != "p99" and t == "lag"),
        ]:
            got = [i for (i,) in con.execute(f"SELECT i FROM t WHERE {views.filter_sql(text)} ORDER BY i").fetchall()]
            self.assertEqual(got, [i for i, (t, s) in enumerate(rows) if keep(t, s)], text)


class Graphs(unittest.TestCase):
    def test_every_saved_views_panels(self):
        for v in views.saved():
            with self.subTest(view=v.name):
                g = views.graph(v)
                json.dumps(g)
                ps = g["panels"]
                self.assertEqual(len({p["id"] for p in ps}), len(ps))
                hover = (v.spec or {}).get("hover") or {}
                self.assertEqual(g["time"], hover.get("x") if ps and hover else None)
                for p in ps:
                    fields = {c["field"] for m in p["marks"] if not m["own_data"] for c in m["channels"]}
                    self.assertLessEqual(set(p["encoded"]), fields | {c["field"] for c in p["facet"]})
                    if p["time"]:
                        self.assertEqual(p["time"], hover["x"])
                        self.assertIn(p["time"], p["encoded"])
                    for f in p["filters"]:
                        self.assertIsNotNone(f["sql"])
        self.assertEqual(views.graph(views.get("experiments"))["panels"], [])
        rt = {p["id"]: p for p in views.graph(views.get("run_timeline"))["panels"]}
        self.assertEqual(len(rt), 10)
        self.assertEqual((rt[LAG]["path"], rt[LAG]["time"], rt[LAG]["x2"], rt[LAG]["encoded"]),
                         ("0", "t_s", None, ["t_s", "value", "series"]))
        self.assertEqual([m["mark"] for m in rt[LAG]["marks"]], ["line", "rule", "text"])
        self.assertEqual([m["own_data"] for m in rt[LAG]["marks"]], [False, True, True])
        self.assertEqual((rt[STALLS]["x2"], rt[STALLS]["encoded"]), ("t2_s", ["t_s", "t2_s", "viewer"]))
        self.assertEqual(rt[STALLS]["filters"], [{"text": "datum.track === 'stall'",
                                                  "sql": "\"track\" IS NOT DISTINCT FROM 'stall'"}])
        # The verdict track's lane and the events' source are the outer spec's calculates.
        self.assertNotIn("lane", rt[VERDICT]["encoded"])
        self.assertEqual(rt["Events (the verdict's in red)"]["encoded"], ["t_s", "series"])
        # Past a pivot only its groupby are columns.
        startup = views.graph(views.get("playback"))["panels"][2]
        self.assertEqual(startup["encoded"], ["cell", "arm"])
        grid = views.graph(views.get("collapse_grid"))["panels"]
        self.assertEqual([(p["id"], p["time"], p["encoded"]) for p in grid], [("0", None, ["seed", "cell", "state"])])
        tree = views.graph(views.get("tree"))["panels"]
        self.assertEqual([p["time"] for p in tree], ["t", "t", "t", None])

    def test_the_legends_take_the_vocabularys_colours(self):
        colours = {n: t["colour"] for s in views.VOCAB_SECTIONS for n, t in vocab.entries(s).items()}
        rt = {p["id"]: p for p in views.graph(views.get("run_timeline"))["panels"]}
        legend = rt[VERDICT]["legend"]
        self.assertEqual((legend["channel"], legend["field"]), ("color", "series"))
        self.assertIn(["after_departure", colours["after_departure"]], legend["values"])
        self.assertEqual([c for _, c in legend["values"]], [colours[n] for n, _ in legend["values"]])
        reasons = views.graph(views.get("tree"))["panels"][2]["legend"]["values"]
        self.assertEqual(reasons, [[n, t["colour"]] for n, t in vocab.entries("reasons").items()])
        self.assertEqual(rt[LAG]["legend"]["values"], [["p50", None], ["p99", None], ["max", None]])

    def test_vocab_references_expand_as_domains_ts_expands_them(self):
        phases, episodes = vocab.entries("phases"), vocab.entries("episodes")
        spec = {"encoding": {"color": {"field": "phase", "scale": {"vocab": "phases", "only": ["steady", "startup"],
                                                                   "zero": False}}}}
        before = json.dumps(spec)
        self.assertEqual(views.expand_vocab(spec), {"encoding": {"color": {"field": "phase", "scale": {
            "zero": False, "domain": ["startup", "steady"],
            "range": [phases["startup"]["colour"], phases["steady"]["colour"]]}}}})
        self.assertEqual(json.dumps(spec), before)
        out = views.expand_vocab({"scale": {"vocab": ["phases", "episodes"], "only": ["collapse", "after_up",
                                                                                     "unhealthy"]}})
        self.assertEqual(out["scale"], {"domain": ["after_up", "unhealthy", "collapse"],
                                        "range": [phases["after_up"]["colour"], episodes["unhealthy"]["colour"],
                                                  episodes["collapse"]["colour"]]})
        out = views.expand_vocab({"vconcat": [{"encoding": {"y": {"sort": {"vocab": "phases"}}}},
                                              {"encoding": {"facet": {"sort": {"vocab": "episodes",
                                                                               "only": ["collapse", "healthy"]}}}}]})
        self.assertEqual(out["vconcat"][0]["encoding"]["y"]["sort"], list(phases))
        self.assertEqual(out["vconcat"][1]["encoding"]["facet"]["sort"], ["healthy", "collapse"])
        same = {"data": {"values": [{"vocab": "phases"}]},
                "layer": [{"mark": "rect", "encoding": {"x": {"datum": {"vocab": 1}}, "y": {"sort": ["a"]}}}]}
        self.assertEqual(views.expand_vocab(same), same)
        for spec, message in [
            ({"scale": {"vocab": "kinds"}}, r"^scale: no vocabulary \"kinds\" \(phases, episodes, reasons\)$"),
            ({"scale": {"vocab": []}}, r"vocab is a vocabulary or a list of them"),
            ({"vconcat": [{"encoding": {"color": {"scale": {"vocab": "phases", "only": ["departure"]}}}}]},
             r"^vconcat\[0\]\.encoding\.color\.scale: \"departure\" is no name of phases$"),
            ({"sort": {"vocab": ["phases", "reasons"], "only": ["open", "x"]}},
             r"\"x\" is no name of phases or reasons"),
            ({"sort": {"vocab": "phases", "only": "steady"}}, r"only is a list of names"),
            ({"sort": {"vocab": "phases", "only": ["steady", "steady"]}}, r"only lists a name twice"),
            ({"sort": {"vocab": "phases", "order": "descending"}}, r"a vocab sort takes no order"),
            ({"scale": {"vocab": "phases", "range": ["#000000"]}}, r"a vocab scale takes no range"),
            ({"scale": {"domain": {"vocab": "phases"}}}, r"^scale\.domain: a vocab reference stands"),
            ({"vocab": "phases"}, r"^spec: a vocab reference stands"),
        ]:
            with self.subTest(spec=spec), self.assertRaisesRegex(ValueError, message):
                views.expand_vocab(spec)


@unittest.skipIf(engine.duckdb is None, "needs the duckdb module (uv)")
class Selections(unittest.TestCase):
    RUN = {"exp": "sf0001", "cell": "star-5", "arm": "a", "seed": "2001"}

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-views-sel-")
        root = Path(cls.tmp.name)
        lake = catalog_fixture(root / "fixture")
        cls.pool = engine.Pool(lake, root / "spill", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        cls.every = cls.rows(cls, "run_timeline", cls.RUN)

    @classmethod
    def tearDownClass(cls):
        cls.pool.close()
        cls.tmp.cleanup()

    def rows(self, name, params, sel=None):
        return views.run(self.pool, name, params, sel=sel).records()

    def check(self, name, params, sel=None):
        return views.check(self.pool, name, params, sel)

    def assertRows(self, got, want):
        """The same rows, in any order: the view orders by track, series
        and t_s, and leaves rows alike in those as they come."""
        key = lambda r: json.dumps(r, sort_keys=True)  # noqa: E731
        self.assertEqual(sorted(map(key, got)), sorted(map(key, want)))

    def test_a_moment_gives_that_seconds_rows_of_every_track(self):
        got = self.rows("run_timeline", self.RUN, {"at": {"t_s": 20}})
        self.assertRows(got, [r for r in self.every if r["t_s"] == 20])
        self.assertEqual({r["track"] for r in got}, {"lag", "heat", "phase", "seed", "copies", "bytes", "data",
                                                     "packets"})

    def test_a_range_on_a_panel_takes_its_filter(self):
        got = self.rows("run_timeline", self.RUN, {"panel": LAG, "at": {"t_s": [17, 20]}})
        self.assertRows(got, [r for r in self.every if r["track"] == "lag" and 17 <= r["t_s"] <= 20])
        self.assertEqual(len(got), 4 * 3)
        # With no panel, a range is the time field's alone.
        got = self.rows("run_timeline", self.RUN, {"at": {"t_s": [17.5, 18.5]}})
        self.assertRows(got, [r for r in self.every if 17.5 <= r["t_s"] <= 18.5])

    def test_a_range_on_the_stall_track_gives_the_stalls_overlapping_it(self):
        stalls = [r for r in self.every if r["track"] == "stall"]
        self.assertEqual([(r["t_s"], r["t2_s"]) for r in stalls], [(18.0, 22.75)] * 5)
        for lo, hi, n in [(20, 21, 5), (10, 17.5, 0), (22.75, 30, 5), (23, 30, 0), (10, 18, 5)]:
            got = self.rows("run_timeline", self.RUN, {"panel": STALLS, "at": {"t_s": [lo, hi]}})
            self.assertEqual(len(got), n, (lo, hi))
            self.assertRows(got, [r for r in stalls if r["t_s"] <= hi and r["t2_s"] >= lo])
        got = self.rows("run_timeline", self.RUN, {"panel": STALLS, "at": {"t_s": [20, 21], "viewer": 3}})
        self.assertRows(got, [r for r in stalls if r["viewer"] == 3])
        self.assertEqual(len(got), 1)
        w = views.where(views.get("run_timeline"), {"panel": "2", "at": {"t_s": [20, 21], "viewer": 3}})
        self.assertEqual(w.parts, ["datum.track === 'stall'", "t_s to t2_s overlaps 20 to 21", "viewer = 3"])
        self.assertEqual(w.unapplied, [])

    def test_a_grid_cell_gives_its_runs(self):
        got = self.rows("collapse_grid", {"exp": "sf0001"}, {"at": {"cell": "star-5", "seed": 2001}})
        self.assertEqual(sorted((r["arm"], r["cell"], r["seed"]) for r in got),
                         [("a", "star-5", 2001), ("off", "star-5", 2001)])
        self.assertEqual(self.check("collapse_grid", {"exp": "sf0001"}, {"at": {"cell": "star-5", "seed": 2001}}), [])

    def test_a_workbench_statement_and_its_chart(self):
        sql = "SELECT cell, seed, arm_label FROM experiment_jobs WHERE exp_id = $exp ORDER BY ALL"
        params = {"sql": sql, "exp": "sf0001", "x": "seed", "y": "cell", "color": "arm_label"}
        got = self.rows(views.WORKBENCH, params, {"at": {"arm_label": "off", "seed": 2002}})
        self.assertEqual([(r["cell"], r["seed"]) for r in got], [("halve-5", 2002), ("star-5", 2002)])
        self.assertEqual(self.check(views.WORKBENCH, params, {"at": {"seed": 2002}}), [])
        self.assertEqual(self.check(views.WORKBENCH, {**params, "y": "lag"}),
                         ["the statement has no column 'lag' for the chart's y: its columns are cell, seed, arm_label"])
        # A saved view's statement, its chart the view's.
        url = views.link(views.WORKBENCH, {"view": "run_timeline", **self.RUN}, {"panel": STALLS, "at": {"viewer": 3}},
                         base=views.BASE)
        name, p, sel = views.parse(url)
        self.assertEqual([r["viewer"] for r in self.rows(name, p, sel)], [3])

    def test_what_check_finds(self):
        self.assertEqual(self.check("run_timeline", self.RUN, {"panel": STALLS, "at": {"t_s": [20, 21]}}), [])
        self.assertEqual(self.check("run_timeline", {**self.RUN, "exp": "nope"}),
                         ["no experiment 'nope' in the catalog"])
        self.assertEqual(self.check("run_timeline", {**self.RUN, "cell": "star-9"}),
                         ["sf0001 has no cell 'star-9': its cells are halve-5, star-5"])
        self.assertEqual(self.check("collapse_diff", {"exp": "sf0001", "arm": "a", "arm2": "b"}),
                         ["sf0001 has no arm2 'b': its arms are a, off"])
        self.assertEqual(self.check("run_timeline", {**self.RUN, "seed": "2009"}),
                         ["seed 2009 didn't run in sf0001 star-5 a: its seeds are 2001, 2002"])
        self.assertEqual(self.check("run_timeline", {**self.RUN, "cell": "halve-5", "seed": "2002"}),
                         ["seed 2002 didn't run in sf0001 halve-5 a: its seeds are 2001"])
        self.assertEqual(self.check("run_timeline", self.RUN, {"panel": STALLS, "at": {"t_s": [30, 35]}}),
                         ["the selection matches no row of run_timeline"])
        self.assertEqual(self.check("run_timeline", {k: v for k, v in self.RUN.items() if k != "seed"}),
                         ["run_timeline: no value for $seed"])
        self.assertRegex(self.check("run_timeline", self.RUN, {"at": {"lane": "x"}})[0], "encodes no column 'lane'")
        self.assertEqual(self.check("nope", {}), ["no view 'nope': " + ", ".join(v.name for v in views.saved())
                                                  + ", or workbench"])


if __name__ == "__main__":
    unittest.main()
