#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.catalog over golden_cli's fixture cache and farm state, and
simlab.verdict's derive pool:

  Index        an index's Progress and Indexed, and one with nothing to
               do; state and availability
  Snapshot     the catalog's latest snapshot: none without a catalog,
               DuckLake's after an index, moved by one that changes the
               catalog and not by one with nothing to do, read throughout
               an index without holding it up, none while the metadata
               stays busy
  Child       an index in a forkserver's child, as the console runs one,
               its derives in a pool of the child's
  Locks        locks other processes hold on the metadata (HOLD): an index
               commits while another process reads throughout (WAL); with
               a rollback journal its commit waits for the reader in
               flight, the switch giving way, and the next index makes it
               WAL; readers (lake, a pool's) wait out BEGIN EXCLUSIVE in a
               rollback journal and aren't held up in WAL; a write the
               lock refused lets go of it and is tried again; an index
               that can't read the catalog's record fails, the catalog
               kept, not made again; two indexes
               at once do the work once; compacting the lock refuses is
               skipped, the index succeeding, and the next index compacts
  Pins         a connection reads one snapshot through an index and its
               compacting (the snapshot it superseded kept, however old;
               older ones let go), and pinned()'s readers share one
               across connections, an index inside reading the latest
  Crash        every commit of an index names only files fsynced with
               their directories, its links among them; a write outside a
               transaction commits by itself; with every file the last
               index wrote emptied, as a crash left the shared catalog,
               reads fail, recorded() and state() raise Damaged naming the
               tables and the repair, and the next index makes them again
               as they were, then has nothing to do, compacting keeping
               what they read; an emptied cache file (its link's) leaves
               its table without its source, saying so, until a new file
               comes; a lock on the record says nothing is damaged
  Derive       the forkserver pool's derived tables are those made in this
               process, with a DuckDB connection open here; a source that
               fails comes back failed, the others judged
  Schema       catalog.schema over an engine.Pool: the console's parts
  Kinds        event_kinds from the vocabulary: its every kind, a row for
               each sim whose runs may hold it, with n 0 where no run wrote
               it; a kind the runs hold that it lacks, or lacks for that
               sim, flagged and warned of; the table docs' kinds and phases
  Fetch        the flushes a fetch pulls (store.pull_sources stood in) and
               what the S3 ledger's new lines cost; its refusals
  Tiers        an experiment's tiers from its cells' names, tests and env,
               its index recording none
  Tables       a sim's tables the catalog has no code for (spec_requests,
               parks), folded by the store, as they are; arrivals' and
               haves' -1 for no node NULL; asks' and haves' truth, NULL
               for a job without arrivals; forecast in either schema
               (tail_ms and extrapolated, or first_sight and lazy), the
               other's columns NULL; parents' order of each node's
               changes, n, NULL where a run's table has none
  Retired      a job of each sim whose events hold retired ids: the
               catalog's events view and the cache's read each as its
               replacement for the job's sim (strims-sim's parent_lost by
               its value), written_kind the id written; a parent_lost in a
               job with the scheduler log on warned of
  Docs         a change to the docs alone (a registry's entry, a column's
               doc in fields.toml, the vocabulary) makes the doc tables
               again and nothing else, describe showing it, the event
               counts kept, then nothing to do; the docs as they were give
               the version they had; an index warns of the columns the
               runs hold that fields.toml doesn't declare
  DocsMain     the shared catalog's docs are main's: an index whose doc
               files are main's records them at main's key; one whose code
               isn't main's (a branch's, the console server's) and whose
               docs differ leaves the doc tables, saying so, indexes its
               data and marks them behind it; one whose docs_version is
               the tables' at main's key makes them again, counting again;
               when main moves, the tables stay until an index whose files
               are main's new ones
  DocsFresh    with no doc tables to keep, an index whose docs aren't
               main's makes its own, not as main's, and the next whose
               files are main's makes them again
  MainDocs     main's doc files from git, for the main checkout's lake
               alone; this checkout's as git names their contents
  DocsVersion  docs_version hashes the code that loaded, and covers every
               name catalogs_sql's code reads
  Registries   every one of store_host's TABLES has its STANDARD_KEYS
               and STANDARD_JOINS, and they name no other; store_host's
               literal TABLES and SAMPLE_TABLES are fields.toml's tables
               and sample tables, in its order, each main writes written by
               both sims; the registries (keys, joins, normalization,
               query.TABLE_KNOBS) and the views' nullification are
               fields.toml's; the two sims' ZSTD lists name the same tables
  Commands     simdb.py's lake --param, view and describe over the
               catalog, as a caller runs them: params bound typed or as
               text, one missing named; the saved views listed, run with
               their defaults or params, an unknown one refused; describe's
               every topic and name (event kinds too), none naming the
               Ask panel's tools

  python3 hack/simlab/tests/test_catalog.py
"""

import concurrent.futures as cf
import contextlib
import copy
import dataclasses
import io
import json
import multiprocessing
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
import time
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

import golden_cli  # noqa: E402
import simlab  # noqa: E402
from simlab import (ask, catalog, describe, engine, forecast, query, semantic, store, store_host, suite,  # noqa: E402
                    verdict, views, vocab)


class Fixture(unittest.TestCase):
    """golden_cli's fixture under a scratch root: the cache, the farm's and
    the suite's state; the main checkout is the root."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-catalog-")
        cls.root = Path(cls.tmp.name)
        saved = os.environ.get("SIM_DB_ROOT")
        try:
            golden_cli.build(cls.root)
        finally:
            os.environ.pop("SIM_DB_ROOT", None)
            if saved is not None:
                os.environ["SIM_DB_ROOT"] = saved

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def setUp(self):
        for p in (mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "target" / "sim-db"), "TZ": "UTC"}),
                  mock.patch.object(simlab.simdb(), "main_checkout", lambda: self.root),
                  mock.patch.object(store_host, "main_checkout", lambda: self.root)):
            p.start()
            self.addCleanup(p.stop)

    def index(self, workers=2):
        events = []
        return catalog.index(workers, events.append), events

    def add_source(self, src):
        """A cache source of one job, folded and exported as a suite run's
        flush is."""
        job = (src, f"k-{src.rsplit('/', 1)[1]}", *golden_cli.JOBS[0][2:])
        d = self.root / "jobs-tables" / src.replace("/", "~")
        meta = golden_cli.job_dir(d, job)
        db = self.root / "jobs-tables" / f"{src.replace('/', '~')}.duckdb"
        store_host.fold(db, [meta], src)
        store_host.export(db, store_host.cache_root() / "cache" / src)


class Index(Fixture):
    def test_an_index_says_what_it_does_and_then_that_theres_nothing_to_do(self):
        got, events = self.index()
        self.assertTrue(got.changed)
        srcs = sorted(catalog.scan_cache())
        self.assertEqual((sorted(got.judged), got.failed), (srcs, {}))
        self.assertEqual(got.sources, srcs)
        self.assertGreater(got.new, 0)
        self.assertEqual((got.replaced, got.gone), (0, 0))
        kinds = [e.kind for e in events]
        self.assertEqual(kinds[0], "scan")
        self.assertEqual(kinds[-1], "done")
        self.assertIn(f"verdicts: {len(srcs)}/{len(srcs)} sources", [e.message for e in events])
        derive = [e for e in events if e.kind == "derive"][-1]
        self.assertEqual((derive.done, derive.total), (len(srcs), len(srcs)))
        self.assertRegex(events[0].message, r"^index: \d+ new files, 0 changed, 0 gone; ")
        again, events = self.index()
        self.assertFalse(again.changed)
        self.assertRegex(events[-1].message, r"^the catalog is up to date \(")
        info = {r["key"]: r["value"] for r in catalog.lake("SELECT * FROM catalog_info;")}
        self.assertEqual(info["verdict_version"], verdict.derived_version())
        self.assertEqual(catalog.state(), {"info": info | {"indexed_at": info["indexed_at"]}, "current": True})
        a = catalog.availability("sf0001")
        self.assertEqual(a.names[:3], ["k", "tbl", "state"])
        self.assertTrue(a.rows)
        self.assertEqual({r[0] for r in a.rows} - {r[0] for r in catalog.availability().rows}, set())

    def test_no_catalog_has_no_state(self):
        with mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "nowhere")}):
            self.assertIsNone(catalog.state())


class Snapshot(Fixture):
    def lakes_latest(self):
        """DuckLake's own latest snapshot id."""
        return catalog.lake("SELECT max(snapshot_id) AS id FROM ducklake_snapshots('lake');")[0]["id"]

    def test_none_without_a_catalog(self):
        with mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.root / "nowhere")}):
            self.assertIsNone(catalog.snapshot())
        self.assertIsNone(catalog.snapshot(self.root / "nowhere"))

    def test_an_index_that_changes_the_catalog_moves_it_and_one_with_nothing_to_do_doesnt(self):
        self.index()
        first = catalog.snapshot()
        self.assertEqual(first["id"], self.lakes_latest())
        self.assertRegex(first["at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\+00:00$")
        self.assertEqual(catalog.snapshot(catalog.lake_root()), first)
        self.assertFalse(self.index()[0].changed)
        self.assertEqual(catalog.snapshot(), first)
        # Reads all through the next index: none holds up its commit.
        self.add_source("suite/wt/snapshot")
        reads, done = [], threading.Event()

        def read():
            while not done.is_set():
                reads.append(catalog.snapshot())
                time.sleep(0.001)
        reader = threading.Thread(target=read)
        reader.start()
        try:
            got = self.index()[0]
        finally:
            done.set()
            reader.join()
        self.assertTrue(got.changed)
        self.assertIn("suite/wt/snapshot", got.sources)
        after = catalog.snapshot()
        self.assertGreater(after["id"], first["id"])
        self.assertEqual(after["id"], self.lakes_latest())
        self.assertTrue(reads)
        self.assertEqual({r["id"] for r in reads if r} - set(range(first["id"], after["id"] + 1)), set())

    def test_a_catalog_busy_past_the_timeout_gives_none(self):
        if catalog.snapshot() is None:
            self.index()
        latest = catalog.snapshot()
        # Exclusive locking mode: nothing else reads it, WAL or not.
        with Held("sole"), mock.patch.object(catalog, "SNAPSHOT_BUSY_S", 0.05):
            self.assertIsNone(catalog.snapshot())
        self.assertEqual(catalog.snapshot(), latest)


def child_index(root):
    """An index in a forkserver's child over the fixture at `root`: its
    Indexed, as a dict, and its Progress events' kinds."""
    os.environ["SIM_DB_ROOT"] = str(Path(root) / "target" / "sim-db")
    simlab.simdb().main_checkout = store_host.main_checkout = lambda: Path(root)
    events = []
    got = catalog.index(2, events.append)
    return dataclasses.asdict(got), [e.kind for e in events]


class Child(Fixture):
    def test_an_index_in_a_forkserver_child(self):
        held = engine.connect()
        self.addCleanup(held.close)
        with cf.ProcessPoolExecutor(1, mp_context=multiprocessing.get_context("forkserver")) as pool:
            got, kinds = pool.submit(child_index, str(self.root)).result(timeout=300)
        self.assertTrue(got["changed"])
        self.assertEqual((sorted(got["judged"]), got["failed"]), (sorted(catalog.scan_cache()), {}))
        self.assertIn("derive", kinds)
        self.assertFalse(catalog.index(0).changed)


# Holds a lock on a SQLite file, in a process of its own, until a line
# comes on its stdin: `kind` read (a read transaction, as a reader's query
# holds one), write (BEGIN IMMEDIATE: the write lock), exclusive (BEGIN
# EXCLUSIVE: no reader either while the journal is rollback's) or sole
# (exclusive locking mode and a write: nothing else reads or writes it,
# whatever the journal).
HOLD = r"""
import sqlite3, sys
s = sqlite3.connect(sys.argv[1], isolation_level=None, timeout=60)
kind = sys.argv[2]
if kind == "sole":
    s.execute("PRAGMA locking_mode=EXCLUSIVE")
if kind == "read":
    s.execute("BEGIN")
    s.execute("SELECT count(*) FROM ducklake_snapshot").fetchall()
else:
    s.execute("BEGIN IMMEDIATE" if kind == "write" else "BEGIN EXCLUSIVE")
    if kind == "sole":
        s.execute("UPDATE ducklake_metadata SET value = value")
print("held", flush=True)
sys.stdin.readline()
s.execute("ROLLBACK")
s.close()
print("released", flush=True)
"""
# How long a held lock outlasts what the sqlite extension waits by itself.
PAST_DEFAULT_S = 6


class Held:
    """HOLD's lock of `kind` on the catalog's metadata, from its start
    until release() (from any thread) or the block's end."""

    def __init__(self, kind):
        self.proc = subprocess.Popen([sys.executable, "-c", HOLD, str(catalog.catalog_path()), kind],
                                     stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)
        self.released_at = None
        self._lock = threading.Lock()
        if self.proc.stdout.readline().strip() != "held":
            self.proc.kill()
            raise AssertionError(f"no {kind} lock held")

    def release(self):
        with self._lock:
            if self.released_at is None:
                self.proc.stdin.write("\n")
                self.proc.stdin.flush()
                said = self.proc.stdout.readline().strip()
                self.proc.wait(timeout=60)
                self.released_at = time.monotonic()
                assert said == "released", said

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        try:
            self.release()
        finally:
            if self.proc.poll() is None:
                self.proc.kill()


def sqlite(sql):
    """The first value `sql` gives on the catalog's metadata, run with
    stdlib sqlite3 in a process of its own: in this one it would be a
    second SQLite beside the lake's connections (catalog.snapshot's
    docs)."""
    code = ("import sqlite3, sys\nc = sqlite3.connect(sys.argv[1])\nrow = c.execute(sys.argv[2]).fetchone()\n"
            "c.commit()\nc.close()\nprint(row[0] if row else '')")
    return subprocess.run([sys.executable, "-c", code, str(catalog.catalog_path()), sql], capture_output=True,
                          text=True, check=True).stdout.strip()


def journal(mode=None):
    """The catalog's journal mode, set to `mode` first where given."""
    got = sqlite("PRAGMA journal_mode" + (f"={mode}" if mode else ""))
    if mode:
        assert got == mode, got
    return got


class Locks(Fixture):
    """Indexes and readers against locks other processes hold on the
    catalog's SQLite metadata: each waits, or tries again, rather than
    fail."""

    def setUp(self):
        super().setUp()
        self.index()
        self.assertEqual(journal(), "wal")

    def count(self, src):
        return catalog.lake(f"SELECT count(*) AS n FROM raw.sources WHERE src = {catalog.q(src)};")[0]["n"]

    def test_an_index_commits_while_another_process_reads_throughout(self):
        self.add_source("suite/wt/locks-read")
        with Held("read"):
            got, _ = self.index()
            self.assertEqual(self.count("suite/wt/locks-read"), 1)
        self.assertTrue(got.changed)
        self.assertIn("suite/wt/locks-read", got.sources)
        self.assertIsNone(got.compact_skipped)

    def test_a_rollback_journals_commit_waits_for_the_reader_in_flight_then_an_index_makes_it_wal(self):
        journal("delete")
        self.add_source("suite/wt/locks-rollback")
        hot = catalog.catalog_path().with_name("catalog.sqlite-journal")
        writing, events, seen = threading.Event(), [], []

        def progress(p):
            events.append(p.message)
            if p.message.startswith("index: writing"):
                writing.set()
        with Held("read") as held:
            def release():
                # The commit's journal while the reader holds the metadata:
                # the commit can't end before the reader does.
                writing.wait(300)
                end = time.monotonic() + 300
                while not hot.exists() and time.monotonic() < end:
                    time.sleep(0.005)
                seen.append((hot.exists(), time.monotonic()))
                time.sleep(PAST_DEFAULT_S)
                held.release()
            t = threading.Thread(target=release)
            t.start()
            try:
                with mock.patch.object(catalog, "WAL_SWITCH_S", 0.2):
                    got = catalog.index(2, progress)
                ended = time.monotonic()
            finally:
                t.join()
        self.assertEqual([s[0] for s in seen], [True])
        self.assertGreaterEqual(ended - seen[0][1], PAST_DEFAULT_S)
        self.assertTrue(got.changed)
        self.assertEqual(self.count("suite/wt/locks-rollback"), 1)
        self.assertFalse([m for m in events if "locked" in m], events)
        # The reader kept the switch from its lock; with none, the next
        # index switches.
        self.assertEqual(journal(), "delete")
        self.assertFalse(self.index()[0].changed)
        self.assertEqual(journal(), "wal")

    def test_readers_wait_out_a_writers_lock(self):
        pool = catalog.pool("test-locks", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        self.addCleanup(pool.close)
        sql = "SELECT count(*) AS n FROM jobs"
        want = catalog.lake(sql + ";")
        journal("delete")
        with Held("exclusive") as held:
            t0 = time.monotonic()
            threading.Timer(PAST_DEFAULT_S, held.release).start()
            with cf.ThreadPoolExecutor(2) as ex:
                lake = ex.submit(catalog.lake, sql + ";")
                pooled = ex.submit(pool.query, sql)
                self.assertEqual((lake.result(), pooled.result().records()), (want, want))
            self.assertGreaterEqual(time.monotonic() - t0, PAST_DEFAULT_S)
        # WAL: a writer's transaction holds up no reader.
        journal("wal")
        pool.refresh()
        with Held("exclusive"):
            t0 = time.monotonic()
            self.assertEqual((catalog.lake(sql + ";"), pool.query(sql).records()), (want, want))
            self.assertLess(time.monotonic() - t0, PAST_DEFAULT_S - 1)

    def test_a_write_the_lock_refuses_lets_go_of_it_and_is_tried_again(self):
        journal("delete")
        probe = "CREATE TABLE raw.locks_probe AS SELECT 1 AS a;"
        self.addCleanup(catalog.lake, "DROP TABLE IF EXISTS raw.locks_probe;", write=True, out=None)
        said = []
        with mock.patch.object(engine, "BUSY_TIMEOUT_S", 0.3), mock.patch.object(catalog, "WRITE_RETRY_S", (1, 1)):
            with Held("read") as held:
                with self.assertRaisesRegex(store_host.DuckError, "database is locked"):
                    catalog.lake(probe, write=True, out=None)
                held.release()
                # The failed commit's lock went with its connection.
                self.assertEqual(catalog.lake("SELECT count(*) AS n FROM raw.info WHERE key = 'catalog_version';"),
                                 [{"n": 1}])
            with Held("read") as held:
                def say(kind, message):
                    # The first try failed under the reader; the next comes
                    # once it's gone.
                    said.append((kind, message))
                    held.release()
                catalog.written(probe, say, "the probe")
        self.assertEqual(catalog.lake("SELECT a FROM raw.locks_probe;"), [{"a": 1}])
        self.assertEqual(said, [("write", "index: the catalog is locked (the probe): trying again in 1 s")])

    def test_an_index_that_cant_read_the_record_fails_and_keeps_the_catalog(self):
        latest, sources = catalog.snapshot(), catalog.lake("SELECT count(*) AS n FROM raw.sources;")
        results = store.results

        def locked(conn, sql, out="json"):
            if "FROM raw.info" in sql:
                raise store_host.DuckError("Failed to query: database is locked")
            return results(conn, sql, out)
        with mock.patch.object(store, "results", locked):
            with self.assertRaisesRegex(store_host.DuckError, "database is locked"):
                self.index()
        self.assertEqual((catalog.snapshot(), catalog.lake("SELECT count(*) AS n FROM raw.sources;")),
                         (latest, sources))

    def test_two_indexes_at_once_do_the_work_once(self):
        src = "suite/wt/locks-twice"
        self.add_source(src)
        lock = catalog.lake_root() / "index.lock"
        with cf.ProcessPoolExecutor(1, mp_context=multiprocessing.get_context("forkserver")) as ex:
            first = ex.submit(child_index, str(self.root))
            end = time.monotonic() + 300
            while True:
                with store_host.locked(lock, block=False) as free:
                    pass
                if not free:
                    break
                self.assertFalse(first.done() or time.monotonic() > end, "the child's index never held the lock")
                time.sleep(0.005)
            self.assertIsNone(catalog.index(0, wait=False))
            second = catalog.index(0)
            got, _ = first.result(timeout=300)
        self.assertTrue(got["changed"])
        self.assertIn(src, got["sources"])
        self.assertFalse(second.changed)
        self.assertEqual(self.count(src), 1)
        self.assertEqual(catalog.lake(f"SELECT count(*) AS n FROM jobs WHERE src = {catalog.q(src)};"), [{"n": 1}])

    def test_compacting_the_lock_refuses_is_skipped_and_the_next_index_compacts(self):
        src = "suite/wt/locks-compact"
        self.add_source(src)
        held, events = [], []

        def progress(p):
            events.append(p.message)
            if p.message.startswith("index: compacting ("):
                held.append(Held("sole"))
        with mock.patch.object(engine, "BUSY_TIMEOUT_S", 0.2), \
                mock.patch.object(catalog, "WRITE_RETRY_S", (0.05, 0.05)):
            try:
                got = catalog.index(2, progress)
            finally:
                for h in held:
                    h.release()
        self.assertEqual(len(held), 1)
        self.assertTrue(got.changed)
        self.assertIn(src, got.sources)
        self.assertEqual(self.count(src), 1)
        self.assertRegex(got.compact_skipped, r"^the catalog stayed locked through 3 tries: .*database is locked")
        self.assertEqual([m for m in events if m.startswith("index: the catalog is locked")],
                         ["index: the catalog is locked (compacting): trying again in 0.05 s"] * 2)
        self.assertIn(f"index: compacting skipped ({got.compact_skipped}); the next index compacts", events)
        self.assertRegex(events[-1], r"^index: done in ")
        self.assertEqual(catalog.skipped_path().read_text(), got.compact_skipped + "\n")
        again, events = self.index()
        self.assertFalse(again.changed)
        self.assertIsNone(again.compact_skipped)
        self.assertEqual([e.message for e in events][:1], ["index: compacting (the last index skipped it)"])
        self.assertRegex(events[-1].message, r"^the catalog is up to date \(")
        self.assertFalse(catalog.skipped_path().exists())
        third, events = self.index()
        self.assertEqual([e.message for e in events if "compacting" in e.message], [])


class Pins(Fixture):
    """Readers at one snapshot while indexes commit and compact: a
    connection's (lake's, a scoring session's), and pinned()'s across
    connections."""

    SQL = ("SELECT (SELECT count(*) FROM raw.sources) AS sources, (SELECT count(*) FROM jobs) AS jobs, "
           "(SELECT id FROM ducklake_current_snapshot('lake')) AS at;")

    def setUp(self):
        super().setUp()
        self.index()

    def snapshots(self):
        return [r["id"] for r in catalog.lake("SELECT snapshot_id AS id FROM ducklake_snapshots('lake') ORDER BY id;")]

    def test_a_connection_reads_one_snapshot_through_an_index_and_its_compacting(self):
        # Every snapshot an hour old: compacting keeps the one the next
        # index supersedes, whatever its age, and lets the others go.
        sqlite("UPDATE ducklake_snapshot SET snapshot_time = '2026-01-01 00:00:00.000000+00'")
        old = self.snapshots()
        with catalog.connected() as conn:
            before = store.results(conn, self.SQL)
            self.add_source("suite/wt/pins-connection")
            got, _ = self.index()
            self.assertTrue(got.changed)
            # jobs, raw.sources and the snapshots as the index found them,
            # their files kept.
            self.assertEqual(store.results(conn, self.SQL), before)
        now = catalog.lake(self.SQL)[0]
        self.assertEqual((now["sources"], now["jobs"]), (before[0]["sources"] + 1, before[0]["jobs"] + 1))
        kept = self.snapshots()
        self.assertEqual([v for v in old if v in kept], [old[-1]])
        self.assertGreater(kept[-1], old[-1])

    def test_pinned_readers_share_one_snapshot_across_connections(self):
        with catalog.pinned() as version:
            before = catalog.lake(self.SQL)
            self.assertEqual(before[0]["at"], version)
            self.add_source("suite/wt/pins-block")
            # The index reads, and writes, the latest.
            got, _ = self.index()
            self.assertIn("suite/wt/pins-block", got.sources)
            with catalog.pinned() as inner:
                self.assertEqual(inner, version)
                self.assertEqual(catalog.lake(self.SQL), before)
            self.assertEqual(catalog.lake(self.SQL), before)
        after = catalog.lake(self.SQL)[0]
        self.assertEqual(after["sources"], before[0]["sources"] + 1)
        self.assertGreater(after["at"], version)


class Crash(Fixture):
    """A crash soon after an index committed: what the OS hadn't written of
    its files is lost, the metadata (fsynced by SQLite) kept."""

    def setUp(self):
        super().setUp()
        self.index()

    def referenced(self):
        """The files the catalog's latest snapshot reads, {path: (schema,
        table)}."""
        out = {}
        for r in catalog.lake("SELECT table_schema AS s, table_name AS t FROM information_schema.tables "
                              "WHERE table_catalog = 'lake' AND table_type = 'BASE TABLE';"):
            for f in catalog.lake(f"SELECT data_file AS d, delete_file AS x FROM ducklake_list_files('lake', "
                                  f"{catalog.q(r['t'])}, schema => {catalog.q(r['s'])});"):
                out |= {os.path.realpath(p): (r["s"], r["t"]) for p in (f["d"], f["x"]) if p}
        return out

    def contents(self):
        """Every table's row count, and the rows of the record and the main
        tables the console reads first."""
        out = {f"{r['s']}.{r['t']}": catalog.lake(f"SELECT count(*) AS n FROM {r['s']}.{r['t']};")[0]["n"]
               for r in catalog.lake("SELECT table_schema AS s, table_name AS t FROM information_schema.tables "
                                     "WHERE table_catalog = 'lake' AND table_type = 'BASE TABLE';")}
        for sql in ("SELECT * EXCLUDE (indexed_at) FROM raw.sources",
                    "SELECT * FROM raw.info WHERE key <> 'indexed_at'", "SELECT * FROM jobs",
                    "SELECT * FROM experiment_jobs", "SELECT * FROM experiments", "SELECT * FROM viewers",
                    "SELECT * FROM verdicts", "SELECT * FROM rows"):
            out[sql] = catalog.lake(sql + " ORDER BY ALL;")
        return out

    def test_an_index_commits_only_files_fsynced_with_their_directories(self):
        self.add_source("suite/wt/crash-durable")
        before = set(catalog.listing())
        fsynced, unsynced, commits = set(), [], []
        fsync, shown = os.fsync, store.shown

        def recorded_fsync(fd):
            fsynced.add(os.path.realpath(os.readlink(f"/proc/self/fd/{fd}")))
            return fsync(fd)

        def checked(conn, script, text=False):
            got = shown(conn, script, text)
            if catalog.ENDS.match(script):
                commits.append(script)
                unsynced.extend(p for p in self.referenced() if p not in before
                                and not {p, os.path.dirname(p)} <= fsynced)
            return got
        with mock.patch.object(os, "fsync", recorded_fsync), mock.patch.object(store, "shown", checked):
            got, _ = self.index()
        self.assertIn("suite/wt/crash-durable", got.sources)
        self.assertGreater(len(commits), 1)
        self.assertEqual(unsynced, [])
        links = {os.path.realpath(r["path"]) for r in catalog.lake(
            "SELECT path FROM raw.files WHERE src = 'suite/wt/crash-durable';")}
        self.assertTrue(links)
        self.assertLessEqual(links, fsynced)

    def test_a_write_outside_a_transaction_commits_by_itself(self):
        with self.assertRaisesRegex(store_host.DuckError, "nowhere"):
            catalog.lake("CREATE TABLE raw.crash_alone AS SELECT 1 AS x; CREATE TABLE raw.crash_too AS "
                         "SELECT * FROM nowhere;", write=True, out=None)
        self.assertEqual(catalog.lake("SELECT x FROM raw.crash_alone;"), [{"x": 1}])
        catalog.lake("DROP TABLE raw.crash_alone;", write=True, out=None)

    def test_an_index_makes_the_tables_a_crash_left_empty_again(self):
        self.add_source("suite/wt/crash-empty")
        before = set(catalog.listing())
        self.index()
        want = self.contents()
        # The crash: what the index wrote of its own (the links are the
        # cache's files) never reached the disk.
        made = self.referenced()
        lost = {p: t for p, t in made.items() if p not in before and "/linked/" not in p}
        self.assertTrue({("raw", "info"), ("raw", "sources"), ("main", "jobs"), ("main", "experiment_jobs")}
                        <= set(lost.values()))
        for p in lost:
            os.truncate(p, 0)
        with self.assertRaisesRegex(store_host.DuckError, "too small to be a Parquet file"):
            catalog.lake("SELECT * FROM experiment_jobs;")
        for read in (catalog.recorded, catalog.state):
            with self.assertRaises(catalog.Damaged) as e:
                read()
            self.assertRegex(str(e.exception), r"^the catalog's files aren't whole: .*raw\.info \(1 file: \S+ is 0 "
                             r"bytes, the catalog says \d+\).*; an index \(python3 hack/sim-db/simdb\.py index\) "
                             "makes their tables again$")
        self.assertEqual(set(catalog.damaged()), set(lost.values()))
        got, events = self.index()
        self.assertEqual(got.repaired, sorted({f"{s}.{t}" for s, t in lost.values()}))
        self.assertIn("index: damaged files, their tables made again: ", "\n".join(e.message for e in events))
        self.assertEqual(catalog.damaged(), {})
        self.assertEqual(self.contents(), want)
        self.assertFalse(self.index()[0].changed)
        # The files only the tables let go read go now: none the tables
        # made again read.
        catalog.compact(keep_minutes=0)
        self.assertFalse(any(os.path.exists(p) for p in lost))
        self.assertEqual(self.contents(), want)

    def test_a_cache_file_a_crash_left_empty_waits_for_its_source(self):
        src = "suite/wt/crash-cache"
        self.add_source(src)
        self.index()
        want = self.contents()
        viewers = lambda: catalog.lake(  # noqa: E731
            "SELECT src, count(*) AS n FROM viewers GROUP BY src ORDER BY src;")
        each = viewers()
        self.assertIn(src, [r["src"] for r in each])
        f = catalog.lake(f"SELECT path, cache_path FROM raw.files WHERE src = {catalog.q(src)} AND tbl = 'viewers';")[0]
        kept = self.root / "crash-cache-viewers.parquet"
        shutil.copyfile(f["cache_path"], kept)
        # The link is the cache's file: both lost.
        os.truncate(f["path"], 0)
        got, events = self.index()
        self.assertEqual(got.repaired, ["raw.viewers"])
        said = "\n".join(e.message for e in events)
        self.assertIn(f"index: 1 cache files aren't whole, left out until their sources are run or fetched again: "
                      f"{f['cache_path']} is 0 bytes", said)
        self.assertEqual(viewers(), [r for r in each if r["src"] != src])
        self.assertEqual(catalog.damaged(), {})
        self.assertFalse(self.index()[0].changed)
        # Its source run again: a file of its own.
        os.replace(kept, f["cache_path"])
        self.assertTrue(self.index()[0].changed)
        self.assertEqual(self.contents(), want)
        catalog.compact(keep_minutes=0)
        self.assertEqual(self.contents(), want)

    def test_a_lock_on_the_record_says_nothing_is_damaged(self):
        results = store.results

        def locked(conn, sql, out="json"):
            if "FROM raw.info" in sql:
                raise store_host.DuckError("Failed to query: database is locked")
            return results(conn, sql, out)
        with mock.patch.object(store, "results", locked):
            with self.assertRaises(store_host.DuckError) as e:
                catalog.recorded()
        self.assertNotIsInstance(e.exception, catalog.Damaged)
        self.assertRegex(str(e.exception), r"^the catalog's record can't be read: its metadata stayed locked past the "
                         r"busy timeout \(Failed to query: database is locked\); nothing is damaged")


class Derive(Fixture):
    def tables(self, srcs):
        name = lambda s: s.replace("/", "~") + ".parquet"  # noqa: E731
        out = {}
        for t in [*verdict.DERIVED, "verdicts"]:
            files = [str(verdict.derived_dir(t) / name(s)) for s in srcs]
            out[t] = store.execute(f"SELECT * FROM read_parquet({verdict.plist(files)}, hive_partitioning=false) "
                                   "ORDER BY ALL;")
        return out

    def test_the_pools_tables_are_this_processs(self):
        srcs = sorted(catalog.scan_cache())
        derived = store_host.cache_root() / "derived"
        held = engine.connect()
        self.addCleanup(held.close)
        shutil.rmtree(derived, ignore_errors=True)
        with mock.patch.object(verdict, "RECYCLE", 2):
            got = verdict.derive(srcs, workers=2)
        self.assertEqual((sorted(got.judged), got.failed), (srcs, {}))
        pooled = self.tables(srcs)
        self.assertTrue(pooled["verdicts"])
        shutil.rmtree(derived)
        here = verdict.derive(srcs, workers=0)
        self.assertEqual(sorted(here.judged), srcs)
        self.assertEqual(self.tables(srcs), pooled)

    def test_a_source_that_fails_comes_back_failed(self):
        bad = store_host.cache_root() / "cache" / "suite" / "wt" / "broken"
        bad.mkdir(parents=True)
        self.addCleanup(shutil.rmtree, bad.parent)
        (bad / "runs.parquet").write_text("not parquet")
        srcs = sorted(catalog.scan_cache())
        said = []
        got = verdict.derive(srcs, workers=2, progress=lambda m, done, total: said.append(m))
        self.assertEqual(list(got.failed), ["suite/wt/broken"])
        self.assertEqual(sorted(got.judged), [s for s in srcs if s != "suite/wt/broken"])
        self.assertTrue(any(m.startswith("verdicts: suite/wt/broken: ") for m in said), said)


class Schema(Fixture):
    def test_the_consoles_parts(self):
        self.index()
        pool = engine.Pool(catalog.lake_root(), self.root / "pool-tmp", size=1)
        self.addCleanup(pool.close)
        got = catalog.schema(pool)
        self.assertEqual(list(got), ["tables", "table_catalog", "metric_catalog", "event_kinds", "experiments",
                                     "catalog_info"])
        names = {(t["schema"], t["name"]) for t in got["tables"]}
        self.assertTrue({("main", "experiments"), ("main", "jobs"), ("raw", "tier_cells")} <= names)
        jobs = next(t for t in got["tables"] if t["name"] == "jobs")
        self.assertEqual(jobs["columns"][0], {"name": "k", "type": "VARCHAR"})
        self.assertEqual([e["exp_id"] for e in got["experiments"]][:1], ["sf0001"])
        self.assertEqual(set(got["experiments"][0]), {"exp_id", "kind", "started_at", "tiers", "arm_labels", "base",
                                                      "question", "cells", "jobs"})
        self.assertNotIn("first_seen", got["metric_catalog"][0])
        self.assertIn("verdict_version", {r["key"] for r in got["catalog_info"]})


def without(doc, kid, sims=None):
    """Vocabulary `doc` without kind `kid`, or with `sims` its sims."""
    out = copy.deepcopy(doc)
    if sims is None:
        del out["kinds"][kid]
    else:
        out["kinds"][kid]["sims"] = sims
    return out


def kind_lines(err):
    """An index's warnings of event kinds, of its stderr `err`."""
    return [s for s in err.getvalue().splitlines() if s.startswith("index: event kind ")]


def undeclared_lines(err):
    """An index's warnings of fields and columns fields.toml doesn't
    declare."""
    return [s for s in err.getvalue().splitlines() if " aren't declared in " in s]


class Kinds(Fixture):
    def test_the_kinds_are_the_vocabularys_and_the_runs(self):
        # A vocabulary that lacks uplink_rate (strims-sim's step jobs write
        # it) and gives latency (every job's) to strims-sim alone.
        doc = without(without(vocab.load(), "uplink_rate"), "latency", ["strims-sim"])
        err = io.StringIO()
        with mock.patch.object(vocab, "load", lambda path=None: doc), contextlib.redirect_stderr(err):
            self.index()
            docs = catalog.kind_docs()
        self.assertEqual(kind_lines(err), [
            "index: event kind latency (ppspp) isn't in the vocabulary, crates/strims-wan/vocab.toml",
            "index: event kind uplink_rate (strims-sim) isn't in the vocabulary, crates/strims-wan/vocab.toml"])
        rows = catalog.lake("SELECT * EXCLUDE (first_seen, last_seen) FROM event_kinds ORDER BY sim, kind;")
        self.assertEqual(list(rows[0]), [*catalog.KIND_COLS, "documented", "n"])
        got = {(r["kind"], r["sim"]): r for r in rows}
        # Every kind of the document's, for each sim that may hold it.
        documented = {k: r for k, r in got.items() if r["documented"]}
        self.assertEqual(sorted(documented), sorted((d["kind"], d["sim"]) for d in docs))
        for d in docs:
            self.assertEqual({c: got[(d["kind"], d["sim"])][c] for c in d}, d)
        self.assertEqual(sorted(k for k, r in got.items() if not r["documented"]),
                         [("latency", "ppspp"), ("uplink_rate", "strims-sim")])
        lost = got[("uplink_rate", "strims-sim")]
        self.assertGreater(lost["n"], 0)
        self.assertEqual({c: lost[c] for c in catalog.KIND_COLS if c not in ("kind", "sim")},
                         dict.fromkeys(set(catalog.KIND_COLS) - {"kind", "sim"}))
        self.assertGreater(got[("latency", "strims-sim")]["n"], 0)
        # The fields as the document gives them: a sim's note, value with
        # its unit, a derived kind's sim.
        join = got[("join", "ppspp")]
        self.assertEqual((join["class"], join["scope"], join["node"], join["notes"], join["value_meaning"]),
                         ("scenario", "viewer", "the viewer", vocab.notes("join", "ppspp"), None))
        # A scenario kind's scope is run unless it names its own; another
        # class's none.
        self.assertEqual([got[(k, "strims-sim")]["scope"] for k in ("latency", "caught_up")], ["run", None])
        self.assertEqual(got[("competing_start", "strims-sim")]["value_meaning"], "the competing flow's round trip, ms")
        self.assertEqual((got[("collapse", "derived")]["class"], got[("collapse", "derived")]["value_meaning"]),
                         ("derived", "its length, s"))
        self.assertGreater(got[("start", "derived")]["n"], 0)
        # A kind no run has written: n 0, for both sims when only other
        # builds write it, its knob named.
        for sim in vocab.SIMS:
            spec = got[("parent_choice", sim)]
            self.assertEqual((spec["n"], spec["env_gate"], spec["details"], spec["documented"]),
                             (0, "SIM_SPEC", ["open", "switch"], True))
        self.assertEqual((got[("arrive", "strims-sim")]["n"], ("arrive", "ppspp") in got), (0, False))
        # A retired kind's row per sim that wrote it, each with that sim's
        # replacement, none written by today's fixture.
        for kid, by in (("uplink", {"strims-sim": "rate_limit", "ppspp": "uplink_rate"}),
                        ("parent_lost", {"strims-sim": "parent_noticed", "ppspp": "parent_departed"}),
                        ("first_frame", {"strims-sim": "first_read"})):
            rows = {s: r for (k, s), r in got.items() if k == kid}
            self.assertEqual({s: r["replaced_by"] for s, r in rows.items()}, by, kid)
            self.assertTrue(all(r["n"] == 0 and r["documented"] for r in rows.values()), kid)
        self.assertGreater(got[("parent_departed", "ppspp")]["n"], 0)
        self.assertIsNone(got[("parent_departed", "ppspp")]["replaced_by"])

    def test_the_docs_name_the_vocabularys_kinds_and_phases(self):
        self.assertEqual(verdict.DERIVED_KINDS, tuple(vocab.derived()))
        self.assertIn(query.TAIL, vocab.phases())
        self.assertEqual(catalog.verdict_phases(), [p for p in vocab.phases() if p != query.TAIL])
        derived, seconds = catalog.TABLE_DOCS["derived_events"][4], catalog.TABLE_DOCS["verdict_seconds"][4]
        for kid in vocab.derived():
            self.assertIn(kid, derived)
        self.assertIn("seed_down (value: the new uplink over the old)", derived)
        for p in catalog.verdict_phases():
            self.assertIn(f"{p} (", seconds)
            self.assertIn(p, catalog.VERDICT_DOCS["phase"])
        self.assertNotIn(query.TAIL, seconds + catalog.VERDICT_DOCS["phase"])

    def test_the_run_metrics_docs_are_metrics_tomls(self):
        # metric_catalog's docs of run_metrics' columns: metrics.toml's
        # run_metrics, one of each scalar run_metrics writes.
        got = catalog.run_metric_docs()
        self.assertEqual(sorted(got), sorted(semantic.run_metric_names()))
        doc = semantic.load()["run_metrics"]
        self.assertEqual(got["collapses"], (doc["collapses"]["description"], None))
        self.assertEqual(got["lag_p99_steady_ms"][1], "ms")


class Fetch(Fixture):
    def test_a_fetch_pulls_the_flushes_the_cache_lacks_and_prices_its_ledger_lines(self):
        self.index()
        lost = f"{golden_cli.FARM}/i-2/3"
        dest = store_host.cache_root() / "cache" / lost
        kept = self.root / "kept-flush"
        shutil.move(dest, kept)
        ledger = self.root / "target" / "sim-farm" / "s3.jsonl"
        calls = []

        def pull_sources(srcs):
            calls.append(sorted(srcs))
            shutil.move(kept, dest)
            use = {"region": "r", "tier1": {}, "tier2": {"pull_get": 1000}, "up": {}, "down": {"internet": 1 << 30},
                   "stored": 0}
            store.farm_ledger(golden_cli.FARM.split("/")[1], use)
            return [store.Pull(golden_cli.FARM.split("/")[1], dest.parents[1], [lost], use)], []

        events = []
        with mock.patch.object(store, "pull_sources", pull_sources):
            got = catalog.fetch("sf0001", progress=events.append)
        self.assertEqual(calls, [[lost]])
        self.assertEqual((got.exp, got.pulled, got.gone, got.ledger_lines), ("sf0001", [lost], [], 1))
        self.assertAlmostEqual(got.s3_usd, 1000 * 0.0044 / 10000 + 0.09, places=9)
        self.assertEqual(events[0], catalog.Progress(
            "pull", "sf0001: pulling the farm runs of 1 flushes the cache lacks", 0, 1))
        self.assertTrue(json.loads(ledger.read_text().splitlines()[-1])["tier2"])
        self.assertEqual(events[-1].kind, "done")

    def test_what_a_fetch_refuses(self):
        self.index()
        with self.assertRaisesRegex(SystemExit, "fetch takes the sample tables"):
            catalog.fetch("sf0001", tables=["bins"])
        with self.assertRaisesRegex(SystemExit, "experiment nope has no jobs"):
            catalog.fetch("nope")


class Tiers(Fixture):
    """Farm-runs indexes of cells without env, their matrices' jobs giving
    it: each cell's tier is the one holding the most of the experiment's
    cells among those with a cell of its name, test and env."""

    def setUp(self):
        super().setUp()
        self.runs = self.root / "target" / "sim-suite" / "farm-runs"
        self.farm = self.root / "target" / "sim-farm"

    def experiment(self, name, cells):
        """A farm run's index naming `cells` (tier cells, each with its
        env or another) by name, sim and test alone, and a job of each."""
        (self.runs / f"{name}.json").write_text(json.dumps({"cells": [
            {"name": c["name"], "sim": c["sim"], "test": c["test"], "why": "w"} for c in cells]}))
        for i, c in enumerate(cells):
            seed = "SIM_SEED" if c["sim"] == suite.PPSPP else "STRIMS_SIM_SEED"
            d = self.farm / f"{name}-{i:02d}"
            d.mkdir()
            (d / "index.jsonl").write_text(json.dumps({
                "id": f"{name}{i}", "key": f"k-{name}-{i}", "arm": "main", "commit": "1" * 40, "test": c["test"],
                "env": {**c["env"], seed: "7"}, "src": f"farm/{name}-00+0more-aaaa/i-1/0"}) + "\n")

    def tiers(self):
        catalog.index(workers=0)
        return {r["exp_id"]: r["tiers"] for r in catalog.lake("SELECT exp_id, tiers FROM experiments;")}

    def test_a_cells_env_decides_its_tier(self):
        by = {}
        for t, cs in suite.TIERS.items():
            for c in cs:
                by.setdefault(c["name"], {})[t] = c
        # A name two tiers give different env, and two cells only one of
        # them holds.
        name, both = next((n, ts) for n, ts in sorted(by.items())
                          if len(ts) >= 2 and len({json.dumps(c["env"], sort_keys=True) for c in ts.values()}) == len(ts))
        a, b = sorted(both)[:2]
        only_b = [c for c in suite.TIERS[b] if c["name"] not in by or set(by[c["name"]]) == {b}][:2]
        self.experiment("sx0001", [both[b], *only_b])
        self.experiment("sx0002", [both[a]])
        self.experiment("sx0003", [{**both[a], "env": {**both[a]["env"], "SIM_SUITE_UNKNOWN": "1"}}])
        got = self.tiers()
        self.assertEqual((got["sx0001"], got["sx0002"], got["sx0003"]), ([b], [a], []))

    def test_the_tier_holding_the_most_of_its_cells(self):
        # Cells of one tier, one of which another tier holds as well (its
        # name and env alike): all of them that tier's.
        shared = None
        for t, cs in sorted(suite.TIERS.items()):
            for c in cs:
                key = (c["name"], json.dumps(c["env"], sort_keys=True))
                others = [u for u, us in suite.TIERS.items() if u != t and any(
                    (x["name"], json.dumps(x["env"], sort_keys=True)) == key for x in us)]
                if others and len(cs) >= 3:
                    shared = (t, c)
                    break
            if shared:
                break
        t, c = shared
        rest = [x for x in suite.TIERS[t] if x["name"] != c["name"]][:2]
        self.experiment("sx0004", [c, *rest])
        self.assertEqual(self.tiers()["sx0004"], [t])


class Tables(Fixture):
    # The two forecast schemas: the HAVE-era one (first_sight, lazy) and
    # its successor (tail_ms, extrapolated); held_ms empty when never held.
    COMMON = [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"), ("peer", "BIGINT"), ("stream", "BIGINT"),
              ("seq", "BIGINT"), ("sample_ms", "BIGINT"), ("mean_ms", "BIGINT"), ("horizon_ms", "BIGINT")]
    FORECASTS = {
        "suite/wt/tail": (COMMON + [("tail_ms", "BIGINT"), ("extrapolated", "BOOLEAN"), ("held_ms", "BIGINT")],
                          [(1000, 1, 0, 2, 0, 5, 60, 50, 80, 120, True, None), (1100, 1, 0, 2, 0, 6, 40, 50, 80, 120,
                                                                              False, 1150)]),
        "suite/wt/lazy": (COMMON + [("first_sight", "BOOLEAN"), ("lazy", "BOOLEAN"), ("held_ms", "BIGINT")],
                          [(1000, 1, 0, 2, 0, 5, 60, 50, 80, False, True, -1)]),
    }

    def fold(self, src, forecast, arrivals=True):
        job = (src, f"k-{src.rsplit('/', 1)[1]}", *golden_cli.JOBS[0][2:])
        d = self.root / "jobs-tables" / src.replace("/", "~")
        meta = golden_cli.job_dir(d, job)
        golden_cli.write_table(d, "forecast", *forecast)
        if "first_sight" in dict(forecast[0]):
            # A parents table without n: two changes in one ms.
            golden_cli.write_table(d, "parents", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("stream", "BIGINT"),
                                                  ("parent", "BIGINT"), ("reason", "VARCHAR")],
                                   [(500, 1, 0, 0, "open"), (500, 1, 0, -1, "shed")])
        golden_cli.write_table(d, "spec_requests", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                    ("held_ms", "BIGINT"), ("held_by_p90", "BOOLEAN")],
                               [(900, 1, 5, None, None), (950, 1, 6, 1150, True)])
        golden_cli.write_table(d, "parks", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                            ("outcome", "VARCHAR")], [(800, 1, 5, "sent")])
        # Node 1's answer from node 2, and the seed's own publish: no source,
        # parent or request, and no viewer.
        if arrivals:
            golden_cli.write_table(d, "arrivals", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("viewer", "BIGINT"),
                                                   ("seq", "BIGINT"), ("how", "VARCHAR"), ("peer", "BIGINT"),
                                                   ("parent", "BIGINT"), ("last_ask_peer", "BIGINT"),
                                                   ("incarnation", "BIGINT")],
                                   [(1150, 1, 0, 6, "answer", 2, 2, 2, 0), (1000, 0, -1, 6, "own", -1, -1, -1, 0)])
        # HAVEs to node 2 of chunk 6 from node 1, and of chunk 7 on a key
        # that named no link.
        golden_cli.write_table(d, "haves", [(c, "BIGINT") for c in ("t_ms", "node", "peer", "seq", "channel_open_ms",
                                                                     "peer_incarnation")],
                               [(1200, 2, 1, 6, 500, 0), (1300, 2, -1, 7, 500, 0)])
        # Node 2's request of chunk 6: of node 1, which held it at 1150,
        # and a rival, node 3, which never did.
        golden_cli.write_table(d, "asks", [(c, "BIGINT") for c in (
            "t_ms", "node", "ask", "peer", "seq", "rtt_ms", "predicted_ms", "predicted_p90_ms", "predicted_p99_ms",
            "channel_open_ms", "since_ms", "peer_incarnation")],
            [(1000, 2, 0, 1, 6, 20, 1100, 1200, 1300, 500, 0, 0), (1000, 2, 0, 3, 6, 20, 1100, 1200, 1300, 500, 0, 0)])
        # The seed's log: decisions about the allocation (no peer, as the
        # sims write it, and an older run's -1), and about node 1: a
        # request, a check of the push of chunk 6 written at 1100 and a
        # CANCEL of chunk 7's push, still queued; its sends: that push, an
        # answer of chunk 6 too, the cancelled push and a push to a peer an
        # older run couldn't name (-1).
        golden_cli.write_table(d, "seed_decisions", [(c, "VARCHAR" if c in ("kind", "why") else "BIGINT") for c in (
            "t_ms", "node", "n", "peer", "peer_incarnation", "kind", "seq", "due_ms", "why", "capacity_bps")],
            [(900, 0, 0, None, None, "window", None, None, "check", None),
             (905, 0, 1, -1, None, "answer_rate", None, None, None, None),
             (950, 0, 2, 1, 0, "request", None, None, None, None),
             (1150, 0, 3, 1, 0, "cancel", 7, None, "queued", None),
             (1200, 0, 4, 1, 0, "check", 6, 1100, None, None)])
        golden_cli.write_table(d, "seed_sends", [(c, "VARCHAR" if c in ("how", "fate") else "BIGINT") for c in (
            "t_ms", "node", "n", "peer", "peer_incarnation", "how", "seq", "fate", "cancel_ms")],
            [(1100, 0, 0, 1, 0, "push", 6, "written", None), (1120, 0, 1, 1, 0, "answer", 6, "written", None),
             (1160, 0, 2, 1, 0, "push", 7, "cancelled", 1150), (1170, 0, 3, -1, None, "push", 8, "closed", None)])
        db = self.root / "jobs-tables" / f"{src.replace('/', '~')}.duckdb"
        store_host.fold(db, [meta], src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
        return meta["k"]

    def test_a_job_without_arrivals_has_no_truth(self):
        # Its asks, HAVEs and seed sends, in a catalog without arrivals,
        # then beside a job with them: their truth unknown, not never held.
        k = self.fold("suite/wt/noarr", self.FORECASTS["suite/wt/tail"], arrivals=False)
        unknown = ([{"peer": p, "held_ms": None, "live": None, "err_ms": None} for p in (1, 3)],
                   [{"peer": 1, "held_ms": None, "live": None}, {"peer": None, "held_ms": None, "live": None}])
        for other in (None, "suite/wt/witharr"):
            if other:
                held = self.fold(other, self.FORECASTS["suite/wt/tail"])
            self.assertEqual(self.index()[0].failed, {})
            asks = f"SELECT peer, held_ms, live, err_ms FROM asks WHERE k = {catalog.q(k)} ORDER BY peer;"
            haves = f"SELECT peer, held_ms, live FROM haves WHERE k = {catalog.q(k)} ORDER BY seq;"
            self.assertEqual((catalog.lake(asks), catalog.lake(haves)), unknown, other)
            sends = f"SELECT count(*) AS n, count(held_ms) AS held FROM seed_sends WHERE k = {catalog.q(k)};"
            self.assertEqual(catalog.lake(sends), [{"n": 4, "held": 0}], other)
        self.assertEqual([r["live"] for r in catalog.lake(f"SELECT live FROM asks WHERE k = {catalog.q(held)} "
                                                          "ORDER BY peer;")], [True, False])
        self.assertEqual([r["held_ms"] for r in catalog.lake(f"SELECT held_ms FROM seed_sends WHERE k = "
                                                             f"{catalog.q(held)} ORDER BY n;")],
                         [1150, 1150, None, None])

    def test_tables_without_code_and_both_forecasts(self):
        keys = {src: self.fold(src, f) for src, f in self.FORECASTS.items()}
        got = self.index()[0]
        self.assertEqual(got.failed, {})
        for t in ("spec_requests", "parks", "arrivals"):
            rows = catalog.lake(f"SELECT k FROM {t} WHERE k IN ({', '.join(catalog.q(k) for k in keys.values())});")
            self.assertEqual(len(rows), len(keys) * (1 if t == "parks" else 2), t)
        held = catalog.lake("SELECT seq, held_ms, held_by_p90 FROM spec_requests WHERE k = "
                            f"{catalog.q(keys['suite/wt/tail'])} ORDER BY seq;")
        self.assertEqual(held, [{"seq": 5, "held_ms": None, "held_by_p90": None},
                                {"seq": 6, "held_ms": 1150, "held_by_p90": True}])
        f = {(r["k"], r["seq"]): r for r in catalog.lake(
            "SELECT k, seq, tail_ms, extrapolated, first_sight, lazy, held_ms FROM forecast WHERE k IN "
            f"({', '.join(catalog.q(k) for k in keys.values())});")}
        tail, lazy = keys["suite/wt/tail"], keys["suite/wt/lazy"]
        self.assertEqual(f[(tail, 5)], {"k": tail, "seq": 5, "tail_ms": 120, "extrapolated": True,
                                        "first_sight": None, "lazy": None, "held_ms": None})
        self.assertEqual(f[(tail, 6)]["held_ms"], 1150)
        self.assertEqual(f[(lazy, 5)], {"k": lazy, "seq": 5, "tail_ms": None, "extrapolated": None,
                                        "first_sight": False, "lazy": True, "held_ms": None})
        names = {r["name"] for r in catalog.lake("SELECT name FROM table_catalog;")}
        self.assertTrue({"spec_requests", "parks", "arrivals", "asks"} <= names)
        # Parents: each node's changes in order, n, and n NULL where a
        # run's table has none.
        parents = catalog.lake("SELECT k, node, n, parent FROM parents WHERE k IN "
                               f"({', '.join(catalog.q(k) for k in keys.values())}) ORDER BY k, node, t_ms, n;")
        self.assertEqual({(r["n"], r["parent"]) for r in parents if r["k"] == lazy}, {(None, 0), (None, None)})
        mine = [(r["node"], r["n"]) for r in parents if r["k"] == tail]
        self.assertTrue(mine)
        self.assertEqual(mine, [(node, i) for node in sorted({x for x, _ in mine})
                                for i in range(sum(1 for x, _ in mine if x == node))])
        cols = {"k": "VARCHAR", "src": "VARCHAR", "t_ms": "BIGINT", "node": "BIGINT", "parent": "BIGINT"}
        self.assertIn("NULL::BIGINT AS n", catalog.view_sql("parents", cols, "parents", {}))
        self.assertNotIn("NULL::BIGINT AS n", catalog.view_sql("parents", {**cols, "n": "BIGINT"}, "parents", {}))
        # The asks view's truth: the peer's hold, from arrivals; a log
        # without a horizon has no held_by_horizon.
        asks = catalog.lake("SELECT peer, held_ms, live, err_ms, pred_err_ms, held_by_p50, held_by_p90, "
                            f"held_by_horizon FROM asks WHERE k = {catalog.q(tail)} ORDER BY peer;")
        self.assertEqual(asks, [{"peer": 1, "held_ms": 1150, "live": True, "err_ms": 140, "pred_err_ms": 50,
                                 "held_by_p50": False, "held_by_p90": True, "held_by_horizon": None},
                                {"peer": 3, "held_ms": None, "live": False, "err_ms": None, "pred_err_ms": None,
                                 "held_by_p50": None, "held_by_p90": None, "held_by_horizon": None}])
        # arrivals' and haves' -1 for no node read NULL (NORMALIZED).
        got = catalog.lake("SELECT node, viewer, peer, parent, last_ask_peer FROM arrivals WHERE k = "
                           f"{catalog.q(tail)} ORDER BY node;")
        self.assertEqual(got, [{"node": 0, "viewer": None, "peer": None, "parent": None, "last_ask_peer": None},
                               {"node": 1, "viewer": 0, "peer": 2, "parent": 2, "last_ask_peer": 2}])
        self.assertIn("last_ask_peer", catalog.NORMALIZED["arrivals"])
        got = catalog.lake(f"SELECT peer, seq, held_ms, live FROM haves WHERE k = {catalog.q(tail)} ORDER BY seq;")
        self.assertEqual(got, [{"peer": 1, "seq": 6, "held_ms": 1150, "live": True},
                               {"peer": None, "seq": 7, "held_ms": None, "live": False}])
        # The seed log's views: a decision about the allocation has no peer,
        # nor has a send an older run couldn't name, and a written push meets
        # its holder's hold in arrivals, as the tables' joins say; their
        # keys and joins are table_catalog's.
        dec = catalog.lake(f"SELECT n, peer, kind FROM seed_decisions WHERE k = {catalog.q(tail)} ORDER BY n;")
        self.assertEqual([(r["n"], r["peer"]) for r in dec], [(0, None), (1, None), (2, 1), (3, 1), (4, 1)])
        sends = catalog.lake(f"SELECT n, peer FROM seed_sends WHERE k = {catalog.q(tail)} ORDER BY n;")
        self.assertEqual([(r["n"], r["peer"]) for r in sends], [(0, 1), (1, 1), (2, 1), (3, None)])
        held = catalog.lake("SELECT s.seq, a.t_ms AS held FROM seed_sends s JOIN arrivals a ON a.k = s.k AND "
                            "a.src = s.src AND a.node = s.peer AND a.seq = s.seq AND a.incarnation = "
                            f"s.peer_incarnation WHERE s.k = {catalog.q(tail)} AND s.how = 'push';")
        self.assertEqual(held, [{"seq": 6, "held": 1150}])
        # The sends' view's truth: node 1's hold of chunk 6 for its push and
        # answer, none of chunk 7, none for a peer an older run couldn't name.
        held = catalog.lake(f"SELECT n, held_ms FROM seed_sends WHERE k = {catalog.q(tail)} ORDER BY n;")
        self.assertEqual([r["held_ms"] for r in held], [1150, 1150, None, None])
        # A cancel's and a check's push, one to one by the documented join:
        # without how and due_ms the check meets the answer of chunk 6 too.
        pairs = catalog.lake(
            "SELECT d.kind, s.n, s.how = 'push' AND (d.kind <> 'check' OR s.t_ms = d.due_ms) AS documented "
            "FROM seed_decisions d JOIN seed_sends s ON s.k = d.k AND s.src = d.src AND s.node = d.node AND "
            "s.peer = d.peer AND s.peer_incarnation = d.peer_incarnation AND s.seq = d.seq "
            f"WHERE d.k = {catalog.q(tail)} ORDER BY d.n, s.n;")
        self.assertEqual([(r["kind"], r["n"], r["documented"]) for r in pairs],
                         [("cancel", 2, True), ("check", 0, True), ("check", 1, False)])
        rows = {r["name"]: r for r in catalog.lake("SELECT name, keys, joins FROM table_catalog WHERE name IN "
                                                   "('seed_decisions', 'seed_sends');")}
        self.assertEqual({t: r["keys"] for t, r in rows.items()},
                         {"seed_decisions": "k, node, n", "seed_sends": "k, node, n"})
        self.assertIn("arrivals.incarnation = seed_sends.peer_incarnation", rows["seed_sends"]["joins"])



class Retired(Fixture):
    """A job of each sim whose events hold retired ids beside current ones:
    the catalog's events view, and the cache's (simlab.query's), read each
    as its replacement for the job's sim (vocab.current), written_kind the
    id it wrote."""

    EVENTS = {
        "suite/wt/old-pp": (0, [(1000, "uplink", 0, -1, 5_000_000, ""), (2000, "parent_lost", 3, 1, 1, "graceful"),
                                (3000, "first_read", 2, -1, 400, "")]),
        "suite/wt/old-ss": (2, [(1000, "up_step", 0, -1, 4_000_000, ""), (1500, "uplink", 0, -1, 3_000_000, ""),
                                (2000, "first_frame", 2, -1, 300, ""), (2500, "parent_lost", 3, 1, 250, ""),
                                (2600, "parent_lost", 4, 1, -1, ""), (3000, "uplink_rate", 0, -1, 2_000_000, "")]),
    }
    # (written, current) of each, in time order.
    WANT = {"suite/wt/old-pp": [("uplink", "uplink_rate", 5_000_000), ("parent_lost", "parent_departed", 1),
                                ("first_read", "first_read", 400)],
            "suite/wt/old-ss": [("up_step", "uplink_rate", 4_000_000), ("uplink", "rate_limit", 3_000_000),
                                ("first_frame", "first_read", 300), ("parent_lost", "parent_noticed", 250),
                                ("parent_lost", "parent_departed", None), ("uplink_rate", "uplink_rate", 2_000_000)]}

    def fold(self, src, job, events):
        k = f"k-{src.rsplit('/', 1)[1]}"
        d = self.root / "jobs-retired" / src.replace("/", "~")
        meta = golden_cli.job_dir(d, (src, k, *golden_cli.JOBS[job][2:]))
        golden_cli.write_table(d, "events", [("t_us", "BIGINT"), ("kind", "VARCHAR"), ("node", "BIGINT"),
                                             ("peer", "BIGINT"), ("value", "BIGINT"), ("detail", "VARCHAR")],
                               [(t * 1000, *e) for t, *e in events])
        db = self.root / "jobs-retired" / f"{src.replace('/', '~')}.duckdb"
        store_host.fold(db, [meta], src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
        return k

    def test_retired_ids_read_as_their_replacements(self):
        keys = {src: self.fold(src, job, events) for src, (job, events) in self.EVENTS.items()}
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(self.index()[0].failed, {})
        for src, k in keys.items():
            got = catalog.lake(f"SELECT written_kind, kind, value FROM events WHERE k = {catalog.q(k)} ORDER BY t_us;")
            self.assertEqual([(r["written_kind"], r["kind"], r["value"]) for r in got], self.WANT[src], src)
            with query.views(query.Selection(src=[src]), derived=False) as v:
                got = v.records("SELECT written_kind, kind, value FROM events ORDER BY t_us;")
            self.assertEqual([(r["written_kind"], r["kind"], r["value"]) for r in got], self.WANT[src], src)
        # The detail of a ppspp parent_lost is its departure's, as parent_departed's.
        self.assertEqual(catalog.lake(f"SELECT detail FROM events WHERE k = {catalog.q(keys['suite/wt/old-pp'])} "
                                      "AND kind = 'parent_departed';"), [{"detail": "graceful"}])
        # What aligns on the events' kind names current ids alone.
        self.assertLessEqual(set(forecast.ALIGN_KINDS), set(vocab.kinds()))

    def test_a_parent_lost_of_a_job_with_the_scheduler_log_warns(self):
        src = "suite/wt/old-spec"
        sim, test, cell, arm, seed, env, *rest = golden_cli.JOBS[2][2:]
        k = f"k-{src.rsplit('/', 1)[1]}"
        d = self.root / "jobs-retired" / src.replace("/", "~")
        meta = golden_cli.job_dir(d, (src, k, sim, test, cell, arm, seed, {**env, "STRIMS_SIM_SPEC": "1"}, *rest))
        golden_cli.write_table(d, "events", [("t_us", "BIGINT"), ("kind", "VARCHAR"), ("node", "BIGINT"),
                                             ("peer", "BIGINT"), ("value", "BIGINT"), ("detail", "VARCHAR")],
                               [(1_000_000, "parent_lost", 3, 1, 250, "")])
        db = self.root / "jobs-retired" / f"{src.replace('/', '~')}.duckdb"
        store_host.fold(db, [meta], src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.index()
        self.assertIn(f"index: 1 parent_lost events in 1 jobs that ran with SIM_SPEC ({k}): a branch older than "
                      "the vocabulary wrote them", err.getvalue())
        # Read as a departure all the same.
        self.assertEqual(catalog.lake(f"SELECT written_kind, kind FROM events WHERE k = {catalog.q(k)};"),
                         [{"written_kind": "parent_lost", "kind": "parent_noticed"}])


class DocsFixture(Fixture):
    """The doc tables' helpers."""

    def described(self, topic):
        """describe's text of `topic`, from readers opened now."""
        pool = catalog.pool("docs", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        try:
            return describe.describe(topic, pool)
        finally:
            pool.close()

    def info(self):
        return {r["key"]: r["value"] for r in catalog.lake("SELECT * FROM catalog_info;")}

    def kinds(self):
        return catalog.lake("SELECT * FROM event_kinds ORDER BY sim, kind;")

    def docs_index(self):
        """An index that makes the doc tables again and nothing else, then
        one with nothing to do: the docs_version it recorded."""
        got, events = self.index()
        self.assertEqual((got.changed, got.docs, got.scoring, got.sources, got.judged), (True, True, False, [], []))
        self.assertEqual([e.kind for e in events], ["done"])
        version = catalog.docs_version()
        self.assertRegex(events[0].message, rf"^index: the docs made again \(docs_version {version}; [0-9.]+ s\)$")
        self.assertEqual(self.info()["docs_version"], version)
        got, events = self.index()
        self.assertFalse(got.changed)
        self.assertRegex(events[-1].message, r"^the catalog is up to date \(")
        return version

    def keys(self, table):
        return catalog.lake(f"SELECT keys FROM table_catalog WHERE name = {catalog.q(table)};")[0]["keys"]


class Docs(DocsFixture):
    MARK = "A docs-only marker."

    def test_a_change_to_the_docs_alone_makes_the_doc_tables_again(self):
        self.index()
        self.assertFalse(self.index()[0].changed)
        first, kinds = self.info()["docs_version"], self.kinds()
        self.assertEqual(first, catalog.docs_version())
        self.assertTrue(any(r["n"] for r in kinds))
        # A registry's entry, as 0d09ff96's keys and joins were.
        with mock.patch.dict(catalog.STANDARD_KEYS, {"viewers": "k, viewer, marker"}):
            keyed = self.docs_index()
            self.assertEqual(catalog.lake("SELECT keys FROM table_catalog WHERE name = 'viewers';"),
                             [{"keys": "k, viewer, marker"}])
            self.assertIn("keys: k, viewer, marker", self.described("viewers"))
            # The runs' counts kept as they were.
            self.assertEqual(self.kinds(), kinds)
        # The entry as it was: the version it had.
        self.assertEqual(self.docs_index(), first)
        self.assertNotIn("k, viewer, marker", self.described("viewers"))
        # A column's doc in a copy of fields.toml: the same text
        # elsewhere is the same version.
        (self.root / "doc-files").mkdir()
        copy = Path(shutil.copy(catalog.FIELDS, self.root / "doc-files" / "fields.toml"))
        with mock.patch.object(catalog, "FIELDS", copy):
            self.assertFalse(self.index()[0].changed)
            entry = '[columns.viewers.churned]\nsims = ["strims-sim", "ppspp"]\ntype = "bool"\ndescription = "'
            text = copy.read_text()
            self.assertEqual(text.count(entry), 1)
            copy.write_text(text.replace(entry, entry + self.MARK + " "))
            edited = self.docs_index()
            desc = catalog.lake("SELECT description FROM metric_catalog WHERE tbl = 'viewers' AND name = 'churned';")
            self.assertTrue(desc[0]["description"].startswith(f"{self.MARK} "), desc)
            self.assertIn(self.MARK, self.described("viewers"))
            # The vocabulary: a kind the runs hold that it lacks keeps its
            # count, undocumented.
            doc = without(vocab.load(), "uplink_rate")
            err = io.StringIO()
            with mock.patch.object(vocab, "load", lambda path=None: doc), contextlib.redirect_stderr(err):
                lacking = self.docs_index()
            self.assertEqual(kind_lines(err), ["index: event kind uplink_rate (strims-sim) isn't in the vocabulary, "
                                               "crates/strims-wan/vocab.toml"])
            was = next(r for r in kinds if (r["kind"], r["sim"]) == ("uplink_rate", "strims-sim"))
            now = next(r for r in self.kinds() if (r["kind"], r["sim"]) == ("uplink_rate", "strims-sim"))
            self.assertGreater(was["n"], 0)
            self.assertEqual((now["documented"], now["description"]), (False, None))
            self.assertEqual({c: now[c] for c in ("n", "first_seen", "last_seen")},
                             {c: was[c] for c in ("n", "first_seen", "last_seen")})
        self.assertEqual(len({first, keyed, edited, lacking}), 4)
        # The docs as they were: their version, and the tables as they were.
        self.assertEqual(self.docs_index(), first)
        self.assertEqual(self.kinds(), kinds)
        self.assertNotIn(self.MARK, self.described("viewers"))

    def test_an_index_warns_of_the_columns_fields_toml_doesnt_declare(self):
        self.index()
        self.assertFalse(self.index()[0].changed)
        first = self.info()["docs_version"]
        # The fixture's runs hold names of its own (golden_cli's), and
        # viewers' declared ones, each with a doc.
        before = catalog.undeclared()
        self.assertNotIn("viewers", before)
        self.assertNotIn("lag_p99_ms", before.get("rows", []))
        docs = {r["name"]: r["description"] for r in catalog.lake(
            "SELECT name, description FROM metric_catalog WHERE tbl = 'viewers';")}
        self.assertTrue(docs["churned"])
        # churned and a row field gone from a copy of fields.toml.
        d = self.root / "undeclared"
        d.mkdir(exist_ok=True)
        copy = Path(shutil.copy(catalog.FIELDS, d / "fields.toml"))
        text = copy.read_text()
        for key in ("[columns.viewers.churned]", "[rows.lag_p99_ms]"):
            i = text.index(key + "\n")
            text = text[:i] + text[text.index("\n\n", i) + 2:]
        copy.write_text(text)
        err = io.StringIO()
        with mock.patch.object(catalog, "FIELDS", copy), contextlib.redirect_stderr(err):
            self.docs_index()
            self.assertEqual(catalog.undeclared(),
                             {**before, "rows": sorted([*before.get("rows", []), "lag_p99_ms"]), "viewers": ["churned"]})
            docs = {r["name"]: r["description"] for r in catalog.lake(
                "SELECT name, description FROM metric_catalog WHERE tbl = 'viewers';")}
            self.assertIsNone(docs["churned"])
        # The warning is undeclared's, a line a table.
        lines = undeclared_lines(err)
        self.assertEqual([s.split("'s ", 1)[0] for s in lines], [f"index: {t}" for t in sorted({*before, "rows",
                                                                                              "viewers"})])
        self.assertIn(f"index: viewers's columns churned aren't declared in {catalog.relative(copy)}", lines)
        self.assertIn("lag_p99_ms", next(s for s in lines if s.startswith("index: rows's fields ")))
        self.assertEqual(self.docs_index(), first)


class DocsMain(DocsFixture):
    # main_docs stood in: the fixture's lake isn't the main checkout's.
    def test_a_checkout_whose_docs_arent_mains_leaves_the_doc_tables(self):
        ids = catalog.docs_ids()
        with mock.patch.object(catalog, "main_docs", lambda: dict(ids)):
            # This checkout's files are main's: the docs are its own, at
            # main's key.
            got, _ = self.index()
            self.assertFalse(got.docs_kept)
            first, kinds = self.info(), self.kinds()
            self.assertEqual((first["docs_version"], first["docs_main"]), (catalog.docs_version(), catalog.docs_key(ids)))
            self.assertNotIn("docs_behind", first)
            self.assertFalse(self.index()[0].changed)
            # A process whose code isn't main's (a branch's, or one older
            # than main's: the console server's), its docs its own.
            other = {**catalog.LOADED_DOCS, "hack/simlab/catalog.py": "0" * 40}
            with mock.patch.object(catalog, "LOADED_DOCS", other), \
                    mock.patch.dict(catalog.STANDARD_KEYS, {"viewers": "k, viewer, branch"}):
                note = catalog.docs_kept({"version": catalog.docs_version()})
                got, events = self.index()
                self.assertEqual((got.changed, got.docs_kept), (False, True))
                self.assertEqual([e.message for e in events][:1], [note])
                self.assertEqual(self.info(), first)
                # Its data indexed, the doc tables left behind it.
                self.add_source("suite/wt/branch")
                got, events = self.index()
                self.assertEqual((got.changed, got.docs_kept, got.sources), (True, True, ["suite/wt/branch"]))
                self.assertIn(note, [e.message for e in events])
                info = self.info()
                self.assertEqual({k: info.get(k) for k in ("docs_version", "docs_main", "docs_behind")},
                                 {"docs_version": first["docs_version"], "docs_main": first["docs_main"],
                                  "docs_behind": "1"})
                self.assertEqual((self.keys("viewers"), self.kinds()), ("k, viewer", kinds))
                got, events = self.index()
                self.assertEqual((got.changed, got.docs_kept), (False, True))
            # One whose docs are main's though its files aren't (its
            # docs_version the doc tables', at main's key): the doc tables
            # made again, the events counted again.
            with mock.patch.object(catalog, "LOADED_DOCS", other):
                got, events = self.index()
                self.assertEqual((got.changed, got.docs, got.docs_kept), (True, True, False))
                info = self.info()
                self.assertNotIn("docs_behind", info)
                self.assertEqual((info["docs_version"], info["docs_main"]), (first["docs_version"], first["docs_main"]))
                n = lambda rows: sum(r["n"] for r in rows)  # noqa: E731
                self.assertGreater(n(self.kinds()), n(kinds))
                self.assertFalse(self.index()[0].changed)
        # Main moves on (its catalog.py another): neither this checkout's
        # files nor the doc tables are main's now, and they stay.
        moved = {**ids, "hack/simlab/catalog.py": "1" * 40}
        with mock.patch.object(catalog, "main_docs", lambda: dict(moved)):
            self.assertFalse(self.index()[0].changed)
            self.assertTrue(catalog.state()["current"])
            # A checkout whose files are main's new ones makes them again,
            # at main's new key.
            with mock.patch.object(catalog, "LOADED_DOCS", {**catalog.LOADED_DOCS, "hack/simlab/catalog.py": "1" * 40}):
                self.assertFalse(catalog.state()["current"])
                got, events = self.index()
                self.assertEqual((got.changed, got.docs), (True, True))
                self.assertEqual(self.info()["docs_main"], catalog.docs_key(moved))
                self.assertFalse(self.index()[0].changed)


class DocsFresh(DocsFixture):
    def test_with_no_doc_tables_to_keep_a_checkout_makes_its_own_until_mains(self):
        other = {**catalog.docs_ids(), "hack/simlab/catalog.py": "1" * 40}
        with mock.patch.object(catalog, "main_docs", lambda: dict(other)):
            got, events = self.index()
            self.assertEqual((got.changed, got.docs_kept), (True, False))
            self.assertIn(f"index: the doc tables made from this checkout's docs (docs_version "
                          f"{catalog.docs_version()}), which aren't main's: there were none to keep",
                          [e.message for e in events])
            info = self.info()
            self.assertEqual(info["docs_version"], catalog.docs_version())
            self.assertFalse({"docs_main", "docs_behind"} & set(info))
            self.assertEqual(self.keys("viewers"), catalog.STANDARD_KEYS["viewers"])
            self.assertFalse(self.index()[0].changed)
        # An index whose files are main's makes them again, as main's.
        with mock.patch.object(catalog, "main_docs", catalog.docs_ids):
            got, events = self.index()
            self.assertEqual((got.changed, got.docs), (True, True))
            self.assertEqual(self.info()["docs_main"], catalog.docs_key(catalog.docs_ids()))


class MainDocs(unittest.TestCase):
    def test_mains_doc_files_from_git_for_the_main_checkouts_lake_alone(self):
        git = lambda *a: store_host.git(*a, cwd=catalog.ROOT)  # noqa: E731
        common = Path(git("rev-parse", "--path-format=absolute", "--git-common-dir")).parent
        with mock.patch.object(catalog, "lake_root", lambda: common / "target" / "sim-db" / "lake"):
            main = catalog.main_docs()
        self.assertIsNotNone(main)
        for p in ("hack/simlab/catalog.py", "hack/simlab/sql/scoring.sql", "hack/sim-suite/suite.py",
                  "crates/strims-wan/vocab.toml", catalog.relative(catalog.FIELDS),
                  catalog.relative(catalog.METRICS_DOC)):
            self.assertEqual(main[p], git("rev-parse", f"main:{p}"), p)
        self.assertTrue(all(catalog.docs_path(p) for p in main))
        # This checkout's: its files as they loaded or as an index reads
        # them, as git names their contents.
        ids = catalog.docs_ids()
        self.assertTrue(all(catalog.docs_path(p) for p in ids))
        for p in ("hack/simlab/catalog.py", "hack/simlab/sql/scoring.sql", catalog.relative(catalog.FIELDS),
                  catalog.relative(catalog.METRICS_DOC)):
            self.assertEqual(ids[p], git("hash-object", p), p)
        # Another lake (a fixture's, SIM_DB_ROOT's elsewhere) has no main
        # to keep its docs for.
        with mock.patch.object(catalog, "lake_root", lambda: Path(tempfile.gettempdir()) / "lake"):
            self.assertIsNone(catalog.main_docs())


class DocsVersion(unittest.TestCase):
    def test_the_version_hashes_the_code_that_loaded(self):
        # As scoring_version and derived_version: a process that outlives
        # an edit to catalog.py hashes the code it runs.
        version = catalog.docs_version()
        with mock.patch("inspect.getsource", lambda f: "an edited file's text"):
            self.assertEqual(catalog.docs_version(), version)

    def test_it_covers_every_name_catalogs_sqls_code_reads(self):
        # Each module-level name the code putting the docs together reads
        # is that code's, a value docs_version hashes, a registry it
        # hashes, or a helper making SQL.
        import ast
        import inspect
        import textwrap
        covered = ({f.__name__ for f in (*catalog.DOCS_CODE, *catalog.DOCS_VALUES)} | set(catalog.DOCS_DATA)
                   | {"q", "ident", "jsonl", "read_jsonl"})
        for f in catalog.DOCS_CODE:
            tree = ast.parse(textwrap.dedent(inspect.getsource(f)))
            own = {a.arg for a in ast.walk(tree) if isinstance(a, ast.arg)} | {
                n.id for n in ast.walk(tree) if isinstance(n, ast.Name) and isinstance(n.ctx, ast.Store)}
            read = {n.id for n in ast.walk(tree) if isinstance(n, ast.Name) and isinstance(n.ctx, ast.Load)} - own
            names = {n for n in read if hasattr(catalog, n) and not inspect.ismodule(getattr(catalog, n))
                     and not isinstance(getattr(catalog, n), type)}
            self.assertEqual(sorted(names - covered), [], f.__name__)


class Registries(unittest.TestCase):
    def test_every_sim_table_has_its_keys_and_joins(self):
        tables = set(store_host.TABLES)
        for name, registry in (("STANDARD_KEYS", catalog.STANDARD_KEYS), ("STANDARD_JOINS", catalog.STANDARD_JOINS)):
            with self.subTest(registry=name):
                self.assertEqual(sorted(tables - set(registry)), [], f"store_host.TABLES without {name}")
                self.assertEqual(sorted(set(registry) - tables), [], f"{name} beyond store_host.TABLES")
                self.assertTrue(all(isinstance(v, str) and v.strip() for v in registry.values()), name)

    def test_the_sample_tables_are_tables(self):
        self.assertEqual(sorted(set(store_host.SAMPLE_TABLES) - set(store_host.TABLES)), [])

    def test_fields_tables_are_tables_both_sims_write(self):
        tables = vocab.fields(catalog.FIELDS)["tables"]
        # store_host's tuples are literal (the farm's hosts run it alone):
        # fields.toml's, in its order.
        self.assertEqual(list(store_host.TABLES), list(tables),
                         "store_host.TABLES isn't fields.toml's tables: register a table on main first")
        self.assertEqual(list(store_host.SAMPLE_TABLES), [t for t, e in tables.items() if e.get("sample")])
        for t, tab in tables.items():
            with self.subTest(table=t):
                self.assertIn(tab["sims"], ([], ["strims-sim", "ppspp"]), "a table one sim writes and the other doesn't")

    def test_both_sims_compress_the_same_tables(self):
        lists = {}
        for sim, path in (("strims-sim", catalog.ROOT / "crates" / "strims-sim" / "src" / "fields.rs"),
                          ("ppspp", catalog.ROOT / "crates" / "strims-ppspp" / "tests" / "sim" / "fields.rs")):
            m = re.search(r"\bZSTD: \[&str; \d+\] = \[(.*?)\];", path.read_text(), re.S)
            self.assertIsNotNone(m, f"{path}: no ZSTD list")
            lists[sim] = sorted(re.findall(r'"([a-z_]+)"', m.group(1)))
        self.assertEqual(lists["strims-sim"], lists["ppspp"], "the sims' ZSTD lists")
        self.assertEqual(sorted(set(lists["ppspp"]) - set(store_host.TABLES)), [])

    def test_the_registries_are_fields_tomls(self):
        tables = vocab.fields(catalog.FIELDS)["tables"]
        self.assertEqual(sorted(set(query.TABLE_KNOBS) - set(store_host.TABLES)), [])
        for t, tab in tables.items():
            with self.subTest(table=t):
                self.assertEqual(sorted(tab.get("knobs", ())), sorted(query.TABLE_KNOBS.get(t, ())))
                self.assertEqual(catalog.STANDARD_KEYS[t], tab["keys"])
                self.assertEqual(catalog.STANDARD_JOINS[t], tab["joins"])
                self.assertEqual(catalog.NORMALIZED.get(t), tab.get("normalized"))
        # The views read -1 as none where fields.toml says a sim writes it
        # so, or older runs did; seconds and bins in every numeric column
        # but the ids.
        self.assertEqual(catalog.VIEW_NONE["arrivals"], {"peer", "parent", "last_ask_peer", "viewer"})
        self.assertEqual(catalog.VIEW_NONE["seed_sends"], {"peer"})
        self.assertEqual(catalog.NONE_ALL, {"seconds", "bins"})
        self.assertEqual(catalog.NONE_NEGATIVE["viewers"], {"arrive_ms"})
        sql = catalog.view_sql("bins", {"k": "VARCHAR", "src": "VARCHAR", "bin": "BIGINT", "zz_old": "BIGINT"},
                               "bins", {}, {})
        self.assertIn('x."bin", nullif(x."zz_old", -1) AS "zz_old"', sql)
        # A table's grain names them, in each sim.
        grain = catalog.field_docs()["tables"]["arrivals"][0]
        self.assertTrue(grain.endswith(", with `STRIMS_SIM_FORECAST=1` or `STRIMS_SIM_SPEC=1` in strims-sim, "
                                       "`SIM_FORECAST=1` or `SIM_SPEC=1` in the ppspp sim"), grain)


class Commands(Fixture):
    def cli(self, *args):
        """simdb.py ARGS as golden_cli runs it: (exit code, stdout, stderr)."""
        p = subprocess.run([sys.executable, str(golden_cli.SIMDB), *args], cwd=self.root,
                           env=golden_cli.environment(self.root), capture_output=True, text=True, timeout=300)
        return p.returncode, p.stdout, p.stderr

    def test_lake_binds_its_params(self):
        self.index()
        code, out, err = self.cli("lake", "SELECT $s AS s, $n AS n, $f AS f, $l AS l, $b AS b, $q AS q", "--param",
                                  "s=sf0001", "--param", "n=7", "--param", "f=2.5", "--param", "l=[1, 2]",
                                  "--param", "b=true", "--param", 'q="007"', "--json")
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(json.loads(out), [{"s": "sf0001", "n": 7, "f": 2.5, "l": [1, 2], "b": True, "q": "007"}])
        code, out, err = self.cli("lake", "SELECT exp_id, jobs FROM experiments WHERE exp_id = $exp", "--param",
                                  "exp=sf0001")
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(out.splitlines()[1:4], ["| exp_id | jobs |", "+--------+------+", "| sf0001 | 7    |"])
        # The same statement as a script, its literal inline, prints the same.
        self.assertEqual(self.cli("lake", "SELECT exp_id, jobs FROM experiments WHERE exp_id = 'sf0001'")[1], out)
        code, out, err = self.cli("lake", "SELECT count(*) AS n FROM structural($exp)", "--param", "exp=sf0001",
                                  "--json")
        self.assertEqual((code, err, len(json.loads(out))), (0, "", 1))

    def test_a_param_missing_is_named(self):
        self.index()
        code, out, err = self.cli("lake", "SELECT * FROM experiments WHERE exp_id = $exp", "--param", "other=1")
        self.assertEqual((code, out, err), (1, "", "lake: no value for $exp\n"))
        code, _, err = self.cli("lake", "SELECT 1", "--param", "nope")
        self.assertEqual(code, 2)
        self.assertIn("'nope' is no NAME=VALUE", err)

    def test_a_failing_query_says_why(self):
        # Both ways a query runs, as a script and with --param, print DuckDB's message and exit 1.
        self.index()
        for extra in ([], ["--param", "exp=sf0001"]):
            code, out, err = self.cli("lake", "SELECT nope FROM experiments", *extra)
            self.assertEqual((code, out), (1, ""), extra)
            self.assertTrue(err.startswith("lake: Binder Error: "), err)
            self.assertNotIn("Traceback", err)

    def test_the_saved_views_listed_run_and_refused(self):
        self.index()
        saved = views.saved()
        code, out, err = self.cli("view")
        self.assertEqual((code, err), (0, ""))
        for v in saved:
            self.assertRegex(out, rf"(?m)^\| {re.escape(v.name)} +\| {re.escape(v.title)} +\| ")
        code, out, _ = self.cli("view", "--json")
        self.assertEqual([v["name"] for v in json.loads(out)], [v.name for v in saved])
        # experiments' $search takes its default, '': every experiment.
        code, out, err = self.cli("view", "experiments", "--json")
        self.assertEqual((code, err), (0, ""))
        self.assertEqual(sorted(r["exp_id"] for r in json.loads(out)), ["sf0001", "suite/wt/inv1", "suite/wt/inv2"])
        code, out, err = self.cli("view", "experiments", "--param", "search=inv2")
        self.assertEqual((code, err), (0, ""))
        self.assertIn("suite/wt/inv2", out)
        self.assertNotIn("sf0001", out)
        code, out, err = self.cli("view", "scorecard", "--param", "exp=sf0001", "--param", "arm=a", "--param",
                                  "arm2=off", "--json")
        self.assertEqual((code, err), (0, ""))
        cells = {r["cell"] for r in catalog.lake("SELECT cell FROM experiment_cells WHERE exp_id = 'sf0001';")}
        self.assertEqual({r["cell"] for r in json.loads(out)}, cells)
        code, out, err = self.cli("view", "experiments", "--max-rows", "1")
        self.assertEqual((code, err), (0, "view experiments: its first 1 rows; --max-rows for more\n"))
        self.assertEqual(len([line for line in out.splitlines() if line.startswith("| ")]), 2)
        code, out, err = self.cli("view", "scorecard", "--param", "exp=sf0001")
        self.assertEqual((code, out), (1, ""))
        self.assertEqual(err, "view scorecard: no value for $arm (its params: exp (exp), arm (arm), arm2 (arm2))\n")
        code, out, err = self.cli("view", "nope")
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("view: no view 'nope': collapse_diff, "), err)

    def test_describe_every_topic_and_name(self):
        self.index()
        code, out, err = self.cli("describe")
        self.assertEqual((code, err), (0, ""))
        pool = catalog.pool("test", size=1, guard=engine.Guard("1GB", 1, "1GiB"))
        self.addCleanup(pool.close)
        self.assertEqual(out, describe.describe(None, pool))
        schema = catalog.schema(pool)
        names = sorted({t["name"] for t in schema["tables"] if t["schema"] == "main"}
                       | {m["name"] for m in describe.macros(schema)} | {v.name for v in views.saved()}
                       | {e["kind"] for e in schema["event_kinds"]})
        self.assertTrue({"experiments", "verdicts", "structural", "effects", "scorecard", "join", "start"}
                        <= set(names))
        for topic in [*describe.TOPICS, *names]:
            got = describe.describe(topic, pool)
            self.assertTrue(got.strip(), topic)
            for tool in (t["name"] for t in ask.TOOLS if "_" in t["name"]):
                self.assertNotRegex(got, rf"\b{tool}\b", topic)
        code, out, err = self.cli("describe", "verdict")
        self.assertEqual((code, out), (1, ""))
        self.assertTrue(err.startswith("describe: no topic 'verdict' (did you mean verdicts"), err)


if __name__ == "__main__":
    unittest.main()
