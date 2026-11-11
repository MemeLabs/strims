#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.engine over a catalog of its own (a DuckLake written as
catalog.py writes one): the pins; the guards (every statement refused,
reads outside the lake, the settings locked); the values against the
console's API (testdata/json_types.json, its JSON for every DuckDB type)
and the CLI (simdb.duck's JSON and -table text); parameters;
the timeout's interrupt and its race with the next query; the pool and
its refresh; scripts as the CLI runs them:

  python3 hack/simlab/tests/test_engine.py
"""

import importlib.util
import json
import re
import sys
import tempfile
import threading
import time
import tomllib
import unittest
from pathlib import Path

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import duckdb  # noqa: E402

from simlab import engine  # noqa: E402

TESTDATA = Path(__file__).with_name("testdata")
# The console API's golden query: a column of each DuckDB type.
TYPES_SQL = (TESTDATA / "json_types.sql").read_text()
API = json.loads((TESTDATA / "json_types.json").read_text())


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


simdb = load(HACK / "sim-db" / "simdb.py", "simdb_for_engine_test")
catalog = load(HACK / "sim-db" / "catalog.py", "catalog_for_engine_test")


def make_lake(lake):
    """A catalog with a few of the tables the server reads, and `nums`
    (100,000 rows)."""
    (lake / "data").mkdir(parents=True)
    w = engine.writer(lake)
    try:
        w.execute("""
CREATE SCHEMA raw;
CREATE TABLE experiments AS SELECT 'e1' AS exp_id, 'farm' AS kind, TIMESTAMPTZ '2026-11-10 10:00:00+00' AS started_at,
  ['robust'] AS tiers, MAP {'base': 'aaaa', 'new': 'bbbb'} AS arms, 2::BIGINT AS cells;
CREATE TABLE raw.tier_cells AS SELECT * FROM (VALUES ('robust', 'calm', MAP {'SIM_VIEWERS': '8'})) t(tier, cell, env);
CREATE TABLE nums AS SELECT range AS i FROM range(100000);""")
    finally:
        w.close()


class Lake(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-engine-")
        cls.dir = Path(cls.tmp.name)
        cls.lake = cls.dir / "lake"
        make_lake(cls.lake)
        cls.pool = engine.Pool(cls.lake, cls.dir / "spill", size=2, guard=engine.Guard("1GB", 2, "1GiB"))

    @classmethod
    def tearDownClass(cls):
        cls.pool.close()
        cls.tmp.cleanup()

    def query(self, sql, params=None, **kw):
        return self.pool.query(sql, params, **kw)

    def status(self, sql, params=None, **kw):
        try:
            self.query(sql, params, **kw)
        except engine.QueryError as e:
            return e.status, str(e)
        return 200, ""


class Pins(unittest.TestCase):
    def test_the_pins_are_simdb_pyprojects_and_the_modules(self):
        self.assertEqual(engine.DUCKDB_VERSION, simdb.DUCKDB_VERSION)
        self.assertEqual("v" + duckdb.__version__, engine.DUCKDB_VERSION)
        deps = tomllib.loads((HACK / "pyproject.toml").read_text())["project"]["dependencies"]
        self.assertIn(f"duckdb=={duckdb.__version__}", deps)
        self.assertIs(catalog.EXTENSIONS, engine.EXTENSIONS)

    def test_the_extensions_are_the_pinned_builds(self):
        engine.ensure_extensions()
        c = engine.connect()
        try:
            engine.check_pins(c)
            saved = dict(engine.EXTENSIONS)
            try:
                engine.EXTENSIONS["ducklake"] = "0000000"
                with self.assertRaisesRegex(engine.PinError, "not the pinned"):
                    engine.check_pins(c)
            finally:
                engine.EXTENSIONS.update(saved)
        finally:
            c.close()


class Guards(Lake):
    def test_queries_and_their_kin_run(self):
        for q in ["SELECT count(*) FROM nums", "EXPLAIN SELECT count(*) FROM nums",
                  "EXPLAIN ANALYZE SELECT count(*) FROM nums", "-- a comment\nEXPLAIN (ANALYZE, FORMAT json) SELECT 1",
                  "/* a block */ DESCRIBE nums", "SUMMARIZE nums", "SHOW TABLES", "FROM nums LIMIT 1",
                  "WITH x AS (SELECT 1 AS a) SELECT * FROM x", "SELECT 1;"]:
            self.assertEqual(self.status(q), (200, ""), q)

    def test_every_statement_but_a_select_is_refused_before_it_binds(self):
        data = self.lake / "data"
        refused = [
            "INSERT INTO nums VALUES (1)",
            "UPDATE nums SET i = 0",
            "DELETE FROM nums",
            "CREATE TABLE t AS SELECT 1",
            "CREATE TEMP TABLE t AS SELECT 1",
            "DROP TABLE nums",
            f"COPY (SELECT 1) TO '{data}/x.csv'",
            f"COPY nums TO '{data}/y.parquet'",
            f"EXPORT DATABASE '{data}/z'",
            "ATTACH ':memory:' AS m",
            f"ATTACH '{data}/w.duckdb' AS m",
            "DETACH lake",
            "INSTALL httpfs",
            "LOAD httpfs",
            "SET threads = 64",
            "SET enable_external_access = true",
            "RESET lock_configuration",
            "USE memory",
            "PRAGMA enable_profiling",
            "CALL ducklake_snapshots('lake')",
            f"EXPLAIN ANALYZE COPY (SELECT 1) TO '{data}/v.csv'",
        ]
        self.assertEqual(len(refused), 21)
        for q in refused:
            status, message = self.status(q)
            self.assertEqual(status, 403, f"{q}: {message}")
            self.assertRegex(message, r"^only SELECT and EXPLAIN SELECT run here, not [A-Z]+", q)
        self.assertEqual(self.status("EXPLAIN SELECT 1; COPY nums TO '/tmp/x.csv'")[0], 403)
        self.assertEqual(self.status("SELECT 1; SELECT 2"), (400, "a query is one statement, not 2"))
        status, message = self.status(f"SELECT * FROM query('COPY (SELECT 1) TO ''{data}/u.csv''')")
        self.assertEqual(status, 400, message)
        self.assertEqual(self.status("SELEC 1")[0], 400)
        for f in ["x.csv", "y.parquet", "z", "w.duckdb", "v.csv", "u.csv"]:
            self.assertFalse((data / f).exists(), f)
        self.assertEqual(self.query("SELECT count(*) AS n FROM nums").values(), [[100000]])

    def test_it_reads_only_the_lakes_files_its_settings_locked(self):
        for q in ["SELECT * FROM read_csv('/etc/passwd')", "SELECT * FROM read_text('/etc/hostname')",
                  "SELECT * FROM glob('/etc/*')", f"SELECT * FROM read_text('{__file__}')"]:
            status, message = self.status(q)
            self.assertEqual(status, 400, q)
            self.assertRegex(message, "Permission Error|disabled", q)
        r = self.query("SELECT current_setting('enable_external_access') AS ext, "
                       "current_setting('lock_configuration') AS lock, "
                       "current_setting('autoload_known_extensions') AS autoload, "
                       "current_setting('threads') AS threads, "
                       "current_setting('memory_limit') AS mem, "
                       "(SELECT readonly FROM duckdb_databases() WHERE database_name = 'lake') AS ro, "
                       "current_database() AS db")
        self.assertEqual(r.values(), [[False, True, False, 2, "953.6 MiB", True, "lake"]])
        files = self.query(f"SELECT count(*) AS n FROM read_parquet('{self.lake / 'data'}/**/*.parquet')")
        self.assertGreater(files.values()[0][0], 0)


class Values(Lake):
    def test_json_is_the_apis(self):
        c = engine.connect()
        c.execute("SET TimeZone = 'America/Los_Angeles'")
        r = engine.run(c, TYPES_SQL)
        j = r.json()
        self.assertEqual([[c["name"], c["type"]] for c in j["columns"]], API["columns"])
        self.assertEqual(json.dumps(j["rows"], separators=(",", ":")), API["rows"])
        t = engine.run(c, TYPES_SQL, temporal_text=True).json()
        self.assertEqual((t["columns"], t["rows"]), (j["columns"], j["rows"]))

    def test_temporal_text_is_duckdbs_where_the_module_cant_hold_the_value(self):
        c = engine.connect()
        c.execute("SET TimeZone = 'America/Los_Angeles'")
        sql = ("SELECT TIMESTAMP_NS '2026-01-01 01:02:03.000000005' AS ns, TIMESTAMP 'infinity' AS inf, "
               "DATE '-infinity' AS ninf, TIMESTAMPTZ '20000-01-01 00:00:00+00' AS far, $t::TIMESTAMPTZ AS t, "
               "[TIMESTAMP '2026-01-01 00:00:00.5'] AS nested, 1 AS ns ORDER BY 1")
        r = engine.run(c, sql, {"t": "2026-07-01 12:00:00+00"}, temporal_text=True)
        self.assertEqual([(col.name, col.type) for col in r.columns], [
            ("ns", "TIMESTAMP_NS"), ("inf", "TIMESTAMP"), ("ninf", "DATE"), ("far", "TIMESTAMP WITH TIME ZONE"),
            ("t", "TIMESTAMP WITH TIME ZONE"), ("nested", "TIMESTAMP[]"), ("ns", "INTEGER")])
        want = ["2026-01-01 01:02:03.000000005", "infinity", "-infinity", "19999-12-31 16:00:00-08",
                "2026-07-01 05:00:00-07", ["2026-01-01 00:00:00.5"], 1]
        self.assertEqual(r.json()["rows"], [want])
        self.assertEqual(r.values(), [want])
        # Without it, the module's values: nanoseconds and the infinities lost.
        self.assertEqual(engine.run(c, sql, {"t": "2026-07-01 12:00:00+00"}).json()["rows"][0][:3],
                         ["2026-01-01 01:02:03", "9999-12-31 23:59:59.999999", "0001-01-01"])
        # What DESCRIBE can't take, or has no times, runs as it is.
        for q in ["DESCRIBE SELECT 1 AS a", "SHOW TABLES", "SUMMARIZE SELECT DATE '2026-01-01' AS d", "SELECT 1 AS a"]:
            a, b = engine.run(c, q, temporal_text=True).json(), engine.run(c, q).json()
            self.assertEqual((a["columns"], a["rows"]), (b["columns"], b["rows"]), q)
        rows = engine.run(c, "SELECT DATE '2026-01-01' + range::INTEGER AS d FROM range(5) ORDER BY d DESC",
                          temporal_text=True).values()
        self.assertEqual(rows, [[f"2026-01-0{i}"] for i in range(5, 0, -1)])
        c.close()

    def test_a_json_column_is_varchar(self):
        r = engine.run(engine.connect(), "SELECT '{\"a\": 1}'::JSON AS j, ['[]'::JSON] AS l")
        self.assertEqual([col.type for col in r.columns], ["VARCHAR", "VARCHAR[]"])
        self.assertEqual(r.json()["rows"], [['{"a": 1}', ["[]"]]])

    def test_values_and_tables_are_the_clis(self):
        c = engine.connect()
        for sql in [TYPES_SQL,
                    "SELECT 1e20::DOUBLE a, 0.1::FLOAT b, 1/3 c, -0.0::DOUBLE d, 1.0::DECIMAL(3,0) e, "
                    "[1.5::DECIMAL(3,1)] f, 7::HUGEINT g, INTERVAL 90 MINUTE h, INTERVAL 3 DAY + INTERVAL 1 SECOND i, "
                    "TIME '01:02:03.25' j, TIMESTAMP '0999-01-02 03:04:05' k",
                    "SELECT 'a' AS x, 'bb' AS x, NULL AS y",
                    "SELECT '' AS empty, ' pad ' AS sp, '中文字符' AS cjk, 'é' AS comb, 'tab\there' AS tab, "
                    "'two\nlines' AS nl",
                    "SELECT range AS i FROM range(3) WHERE false",
                    "SELECT * FROM (VALUES (1, 'x'), (2, NULL)) t(n, s)"]:
            # As JSON: NaN is NaN there.
            self.assertEqual(json.dumps(engine.run(c, sql).records()), json.dumps(simdb.duck(sql + ";")), sql)
            self.assertEqual(engine.run(c, sql, text=True).table(), simdb.duck(sql + ";", out="text"), sql)
        c.close()

    def test_type_names_quote_struct_fields_at_any_depth(self):
        r = engine.run(engine.connect(), "SELECT MAP {'k': {'a b': [1]}} AS m, [{'x': {'y': 1}}] AS l, "
                                         "{'a': [1, 2]::INTEGER[2]} AS s")
        self.assertEqual([c.type for c in r.columns], ['MAP(VARCHAR, STRUCT("a b" INTEGER[]))',
                                                       'STRUCT("x" STRUCT("y" INTEGER))[]',
                                                       'STRUCT("a" INTEGER[2])'])

    def test_rows_stop_at_max_rows(self):
        r = self.query("SELECT i FROM nums ORDER BY i", max_rows=10)
        self.assertEqual((len(r.rows), r.truncated), (10, True))
        r = self.query("SELECT i FROM nums LIMIT 10", max_rows=10)
        self.assertEqual((len(r.rows), r.truncated), (10, False))


class Parameters(Lake):
    def test_they_bind_by_name_whole_numbers_as_bigint(self):
        r = self.query("SELECT $exp || '!' AS s, $seed + 1 AS n, typeof($seed) AS t, $xs AS xs, typeof($f) AS f, "
                       "typeof($whole) AS w, typeof($big) AS b, $yes AS y, $none IS NULL AS z, typeof($ns) AS ns",
                       {"exp": "e1", "seed": 2001, "xs": ["a", "b"], "f": 2.5, "whole": 2.0, "big": 2 ** 60,
                        "yes": True, "none": None, "ns": [1, 2], "unused": 1})
        self.assertEqual(r.values(), [["e1!", 2002, "BIGINT", ["a", "b"], "DOUBLE", "BIGINT", "DOUBLE", True, True,
                                       "INTEGER[]"]])

    def test_a_missing_or_unbindable_one_fails(self):
        self.assertEqual(self.status("SELECT $exp", {}), (400, "no value for $exp"))
        self.assertEqual(self.status("SELECT $exp", {"exp": {"a": 1}}),
                         (400, "$exp is no string, number, boolean or list"))
        self.assertEqual(self.status("SELECT $a, $b", {"b": 1}), (400, "no value for $a"))

    def test_a_list_takes_numbers_and_the_strings_of_the_rest(self):
        r = self.query("SELECT $xs AS xs", {"xs": ["a", True, None, 3]})
        self.assertEqual(r.values(), [[["a", "true", "null", "3"]]])


class Timeout(Lake):
    LONG = "SELECT sum(hash(i)) FROM range(100000000000) t(i)"

    def test_a_long_query_is_interrupted_and_its_connection_works_after(self):
        t0 = time.time()
        status, message = self.status(self.LONG, timeout_s=1)
        self.assertEqual((status, message), (408, "interrupted at the timeout (1 s)"))
        self.assertLess(time.time() - t0, 5)
        for _ in range(3):
            self.assertEqual(self.query("SELECT 1 AS x").values(), [[1]])

    def test_a_streaming_query_is_interrupted_while_it_fetches(self):
        c = engine.connect()
        with self.assertRaises(engine.QueryError) as cm:
            engine.run(c, "SELECT range FROM range(20000000000)", max_rows=10 ** 9, timeout_s=0.5)
        self.assertEqual(cm.exception.status, 408)
        self.assertEqual(engine.run(c, "SELECT 7 AS x").values(), [[7]])
        c.close()

    def test_an_interrupt_never_reaches_the_next_query(self):
        # Timeouts about as long as the query itself: when one fires just
        # as its query ends, the next query on the connection must run.
        c = engine.connect()
        sql = "SELECT sum(range) AS s FROM range(200000)"
        took = []
        for _ in range(5):
            start = time.perf_counter()
            engine.run(c, sql)
            took.append(time.perf_counter() - start)
        each = max(min(took), 0.0005)
        timed_out = 0
        for n in range(300):
            try:
                engine.run(c, sql, timeout_s=each * (0.5 + (n % 7) / 4))
            except engine.QueryError as e:
                self.assertEqual(e.status, 408)
                timed_out += 1
            self.assertEqual(engine.run(c, sql).values(), [[19999900000]])
        c.close()
        self.assertGreater(timed_out, 0)

    def test_a_closed_deadline_never_fires(self):
        c = engine.connect()
        d = engine.Deadline(c, 0.05)
        d.close()
        time.sleep(0.1)
        self.assertFalse(d.fired)
        self.assertFalse(d._timer.is_alive())
        d = engine.Deadline(c, 0.01)
        time.sleep(0.05)
        d.close()
        self.assertTrue(d.fired)
        self.assertEqual(engine.run(c, "SELECT 1 AS x").values(), [[1]])
        c.close()


class Pool(Lake):
    def test_a_query_waits_for_a_free_reader(self):
        held, release = [], threading.Event()

        def hold():
            with self.pool.connection() as conn:
                held.append(conn)
                release.wait(10)

        holders = [threading.Thread(target=hold) for _ in range(2)]
        for t in holders:
            t.start()
        while len(held) < 2:
            time.sleep(0.01)
        done = []
        waiter = threading.Thread(target=lambda: done.append(self.query("SELECT 42 AS x").values()))
        waiter.start()
        time.sleep(0.3)
        self.assertEqual(done, [])
        release.set()
        waiter.join(10)
        for t in holders:
            t.join(10)
        self.assertEqual(done, [[[42]]])

    def test_a_refresh_lets_running_queries_finish_on_theirs(self):
        self.query("SELECT 1")
        before = self.pool.generation
        old_tmp = self.pool._gen.tmp
        with self.pool.connection() as conn:
            self.pool.refresh()
            # The next query opens new readers, while this one still works.
            self.assertEqual(self.query("SELECT 2 AS x").values(), [[2]])
            self.assertEqual(self.pool.generation, before + 1)
            self.assertEqual(engine.run(conn, "SELECT 3 AS x", guard=True).values(), [[3]])
            self.assertTrue(old_tmp.exists())
        self.assertFalse(old_tmp.exists())
        self.assertEqual(self.query("SELECT 4 AS x").values(), [[4]])
        self.assertEqual(self.pool.generation, before + 1)

    def test_a_refresh_while_readers_open_opens_them_again(self):
        pool = engine.Pool(self.lake, self.dir / "spill2", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        opened = pool._open
        calls = []

        def racing():
            gen = opened()
            if not calls:
                pool.refresh()
            calls.append(gen)
            return gen

        pool._open = racing
        try:
            self.assertEqual(pool.query("SELECT 5 AS x").values(), [[5]])
            self.assertEqual(len(calls), 2)
            self.assertFalse(calls[0].tmp.exists())
        finally:
            pool.close()
        self.assertEqual(pool._gen, None)
        with self.assertRaises(engine.QueryError) as cm:
            pool.query("SELECT 1")
        self.assertEqual(cm.exception.status, 503)

    def test_new_tables_reach_the_readers_after_a_refresh(self):
        w = engine.writer(self.lake)
        try:
            w.execute("CREATE OR REPLACE TABLE added AS SELECT 1 AS a")
        finally:
            w.close()
        self.pool.refresh()
        self.assertEqual(self.query("SELECT a FROM added").values(), [[1]])


class Scripts(unittest.TestCase):
    def test_a_script_shows_what_the_cli_shows(self):
        sql = ("CREATE TABLE t AS SELECT range AS i FROM range(3); INSERT INTO t VALUES (9); SET threads = 2; "
               "SELECT i, i * 2 AS j FROM t ORDER BY i; PRAGMA version; SELECT 'x' AS s;")
        c = engine.connect()
        got = "".join(r.table() for r in engine.script(c, sql, text=True))
        c.close()
        self.assertEqual(re.sub(r"[0-9a-f]{10}", "SOURCE", got),
                         re.sub(r"[0-9a-f]{10}", "SOURCE", simdb.duck(sql, out="text")))

    def test_parameters_bind_statement_by_statement(self):
        c = engine.connect()
        got = engine.script(c, "SELECT $a AS a; SELECT $b AS b, $a AS a2", {"a": 1, "b": "x"})
        self.assertEqual([r.values() for r in got], [[[1]], [["x", 1]]])
        c.close()


if __name__ == "__main__":
    unittest.main()
