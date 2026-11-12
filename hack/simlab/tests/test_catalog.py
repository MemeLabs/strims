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
               parks, arrivals), folded by the store, as they are; forecast
               in either schema (tail_ms and extrapolated, or first_sight
               and lazy), the other's columns NULL
  Retired      a job of each sim whose events hold retired ids: the
               catalog's events view and the cache's read each as its
               replacement for the job's sim, written_kind the id written
  Docs         a change to the docs alone (a registry's entry, a doc row
               of a sim's file, the vocabulary) makes the doc tables again
               and nothing else, describe showing it, the event counts
               kept, then nothing to do; the docs as they were give the
               version they had
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
               and STANDARD_JOINS, and they name no other; SAMPLE_TABLES
               are TABLES; the two sims' doc tables (doc_tables) document
               the same tables, each of them one of TABLES, and their
               grains the knobs query.TABLE_KNOBS gives; the two sims'
               ZSTD lists name the same tables
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
from simlab import (ask, catalog, describe, engine, forecast, query, store, store_host, suite, verdict,  # noqa: E402
                    views, vocab)


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


def gap_lines(err):
    """An index's warnings of columns without a doc of their own."""
    return [s for s in err.getvalue().splitlines() if "'s columns without a doc of their own" in s]


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

    def test_the_run_metrics_docs_are_their_docstrings(self):
        # metric_catalog's docs of run_metrics' columns, whether the
        # compiler dedented the docstring (Python 3.13 on) or kept it
        # indented.
        got = catalog.run_metric_docs()
        self.assertEqual(got["collapses"], "collapse spells (`episodes`)")
        self.assertTrue(got["lag_p99_startup_ms"].startswith("the p99 of the honest viewers' lag over their seconds "
                                                             "of startup"), got)
        self.assertTrue(got["lag_p99_steady_ms"].endswith("of steady state (`phase_lags`)"), got)
        first, *rest = verdict.run_metrics.__doc__.splitlines()
        indented = "\n".join([first, *("    " + line if line else line for line in rest)])
        with mock.patch.object(verdict.run_metrics, "__doc__", indented):
            self.assertEqual(catalog.run_metric_docs(), got)


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

    def fold(self, src, forecast):
        job = (src, f"k-{src.rsplit('/', 1)[1]}", *golden_cli.JOBS[0][2:])
        d = self.root / "jobs-tables" / src.replace("/", "~")
        meta = golden_cli.job_dir(d, job)
        golden_cli.write_table(d, "forecast", *forecast)
        golden_cli.write_table(d, "spec_requests", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                                    ("held_ms", "BIGINT"), ("held_by_p90", "BOOLEAN")],
                               [(900, 1, 5, None, None), (950, 1, 6, 1150, True)])
        golden_cli.write_table(d, "parks", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                            ("outcome", "VARCHAR")], [(800, 1, 5, "sent")])
        golden_cli.write_table(d, "arrivals", [("t_ms", "BIGINT"), ("node", "BIGINT"), ("seq", "BIGINT"),
                                               ("how", "VARCHAR"), ("peer", "BIGINT"), ("incarnation", "BIGINT")],
                               [(1150, 1, 6, "answer", 2, 0)])
        # Node 2's request of chunk 6: of node 1, which held it at 1150,
        # and a rival, node 3, which never did.
        golden_cli.write_table(d, "asks", [(c, "BIGINT") for c in (
            "t_ms", "node", "ask", "peer", "seq", "rtt_ms", "predicted_ms", "predicted_p90_ms", "predicted_p99_ms",
            "channel_open_ms", "since_ms", "peer_incarnation")],
            [(1000, 2, 0, 1, 6, 20, 1100, 1200, 1300, 500, 0, 0), (1000, 2, 0, 3, 6, 20, 1100, 1200, 1300, 500, 0, 0)])
        db = self.root / "jobs-tables" / f"{src.replace('/', '~')}.duckdb"
        store_host.fold(db, [meta], src)
        store_host.export(db, store_host.cache_root() / "cache" / src)
        return meta["k"]

    def test_tables_without_code_and_both_forecasts(self):
        keys = {src: self.fold(src, f) for src, f in self.FORECASTS.items()}
        got = self.index()[0]
        self.assertEqual(got.failed, {})
        for t in ("spec_requests", "parks", "arrivals"):
            rows = catalog.lake(f"SELECT k FROM {t} WHERE k IN ({', '.join(catalog.q(k) for k in keys.values())});")
            self.assertEqual(len(rows), len(keys) * (2 if t == "spec_requests" else 1), t)
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
        # The asks view's truth: the peer's hold, from arrivals.
        asks = catalog.lake("SELECT peer, held_ms, live, err_ms, pred_err_ms, held_by_p50, held_by_p90 FROM asks "
                            f"WHERE k = {catalog.q(tail)} ORDER BY peer;")
        self.assertEqual(asks, [{"peer": 1, "held_ms": 1150, "live": True, "err_ms": 140, "pred_err_ms": 50,
                                 "held_by_p50": False, "held_by_p90": True},
                                {"peer": 3, "held_ms": None, "live": False, "err_ms": None, "pred_err_ms": None,
                                 "held_by_p50": None, "held_by_p90": None}])



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
    WANT = {"suite/wt/old-pp": [("uplink", "uplink_rate"), ("parent_lost", "parent_lost"),
                                ("first_read", "first_read")],
            "suite/wt/old-ss": [("up_step", "uplink_rate"), ("uplink", "rate_limit"), ("first_frame", "first_read"),
                                ("parent_lost", "parent_lost"), ("parent_lost", "parent_lost"),
                                ("uplink_rate", "uplink_rate")]}

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
        self.assertEqual(self.index()[0].failed, {})
        for src, k in keys.items():
            got = catalog.lake(f"SELECT written_kind, kind FROM events WHERE k = {catalog.q(k)} ORDER BY t_us;")
            self.assertEqual([(r["written_kind"], r["kind"]) for r in got], self.WANT[src], src)
            with query.views(query.Selection(src=[src]), derived=False) as v:
                got = v.records("SELECT written_kind, kind FROM events ORDER BY t_us;")
            self.assertEqual([(r["written_kind"], r["kind"]) for r in got], self.WANT[src], src)
        # What aligns on the events' kind names current ids alone.
        self.assertLessEqual(set(forecast.ALIGN_KINDS), set(vocab.kinds()))


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
        # A doc row of a sim's file, in a copy of it: the same text
        # elsewhere is the same version.
        copies = []
        (self.root / "doc-files").mkdir()
        for p in catalog.DOC_FILES:
            copies.append(Path(shutil.copy(p, self.root / "doc-files" / p.name)))
        with mock.patch.object(catalog, "DOC_FILES", tuple(copies)):
            self.assertFalse(self.index()[0].changed)
            row = "/// | `viewers` | viewer | `viewer`, "
            text = copies[1].read_text()
            self.assertEqual(text.count(row), 1)
            copies[1].write_text(text.replace(row, f"/// | `viewers` | viewer | {self.MARK} `viewer`, "))
            edited = self.docs_index()
            desc = catalog.lake("SELECT description FROM table_catalog WHERE name = 'viewers';")[0]["description"]
            self.assertTrue(desc.startswith(f"ppspp: {self.MARK} `viewer`, "), desc)
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

    def test_an_index_warns_of_columns_without_a_doc_of_their_own(self):
        self.index()
        self.assertFalse(self.index()[0].changed)
        first = self.info()["docs_version"]
        self.assertIn("churned", {r["name"] for r in catalog.lake(
            "SELECT name FROM metric_catalog WHERE tbl = 'viewers';")})
        gaps = catalog.column_gaps()
        # The fixture's tables as main's docs leave them: no table only a
        # branch documents, and viewers' churned its own.
        self.assertFalse(set(gaps) - {*catalog.doc_tables()["strims-sim"], *catalog.VERDICT_TABLES})
        self.assertNotIn("churned", [c for cs in gaps.get("viewers", {}).values() for c in cs])
        # churned's own words gone from both sims' doc rows, in copies.
        d = self.root / "gap-files"
        d.mkdir(exist_ok=True)
        copies = [Path(shutil.copy(p, d / p.name)) for p in catalog.DOC_FILES]
        for path, own in ((copies[0], "`churned` (it restarted)"), (copies[1], "`churned` (it left at least once)")):
            text = path.read_text()
            self.assertEqual(text.count(own), 1, own)
            path.write_text(text.replace(own, "`churned`"))
        err = io.StringIO()
        with mock.patch.object(catalog, "DOC_FILES", tuple(copies)), contextlib.redirect_stderr(err):
            self.docs_index()
            edited = catalog.column_gaps()
        self.assertTrue(any("churned" in cs for cs in edited["viewers"].values()), edited["viewers"])
        self.assertEqual({v: g for v, g in edited.items() if v != "viewers"},
                         {v: g for v, g in gaps.items() if v != "viewers"})
        # The warning is column_gaps', a line a table.
        lines = gap_lines(err)
        self.assertEqual([s.split("'s columns", 1)[0] for s in lines], [f"index: {v}" for v in edited])
        viewers = next(s for s in lines if s.startswith("index: viewers's "))
        self.assertIn("churned", viewers)
        self.assertIn("gap-files", viewers)
        self.assertEqual(self.docs_index(), first)


class ColumnDocs(unittest.TestCase):
    CELL = ("`a` (one), `b` and `c` (both), `d`, `e` (`silent` (a value), `f`), `g` (the first), "
            "`h`: the rest, `i` (eye), `j` (`h`)")

    def test_a_cells_own_docs_are_its_top_level_names_with_their_meanings(self):
        own = catalog.own_docs(self.CELL)
        self.assertEqual(own, {"a": ("paren", "one"), "c": ("paren", "both"), "e": ("paren", "`silent` (a value), `f`"),
                               "g": ("paren", "the first"), "h": ("colon", "the rest, `i` (eye), `j` (`h`)"),
                               "i": ("paren", "eye"), "j": ("paren", "`h`")})
        # column_docs gives a nested name the text after it too.
        self.assertEqual(catalog.column_docs(self.CELL)["silent"], "a value")

    def test_a_column_without_its_own_doc_is_a_gap_by_what_it_has(self):
        docs = {"strims-sim": {"t": ("a row", self.CELL), "w": ("a row", "`x` (ex)")},
                "ppspp": {"t": ("a row", "`d` (dee), `k`")}}
        cols = {c: "text" for c in "acdeghij"} | {"b": "a sentence", "silent": "a value", "k": None}
        tables = {"t": ("t", cols), "w": ("w", {"x": "ex"}), "u": ("u", {"y": None}),
                  "verdicts": (None, {"onset": "doc", "lag_x": None, "lag_y": "a sentence"})}
        self.assertEqual(catalog.doc_gaps(docs, tables, {"onset": None}), {
            # t: b documented nowhere of its own (its text a sentence's), k
            # by nothing, silent only inside e's parentheses, h's colon doc
            # taking in i's and j's; d is the ppspp sim's own. u, which no
            # doc table documents, is left out, and w has no gap.
            "t": {"none": ["k"], "sentence": ["b"], "nested": ["silent"], "runs on": ["h"]},
            "verdicts": {"none": ["lag_x"], "sentence": ["lag_y"]}})


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
                  "crates/strims-wan/vocab.toml", *(catalog.relative(f) for f in catalog.DOC_FILES)):
            self.assertEqual(main[p], git("rev-parse", f"main:{p}"), p)
        self.assertTrue(all(catalog.docs_path(p) for p in main))
        # This checkout's: its files as they loaded or as an index reads
        # them, as git names their contents.
        ids = catalog.docs_ids()
        self.assertTrue(all(catalog.docs_path(p) for p in ids))
        for p in ("hack/simlab/catalog.py", "hack/simlab/sql/scoring.sql", catalog.relative(catalog.DOC_FILES[1])):
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

    def test_both_sims_document_the_same_tables(self):
        docs = catalog.doc_tables()
        self.assertEqual(sorted(docs), ["ppspp", "strims-sim"])
        self.assertEqual(sorted(set(docs["strims-sim"]) ^ set(docs["ppspp"])), [],
                         "a table one sim's doc table documents and the other's doesn't")

    def test_every_documented_table_is_a_table(self):
        documented = {t for tables in catalog.doc_tables().values() for t in tables}
        self.assertEqual(sorted(documented - set(store_host.TABLES)), [],
                         "the sims document tables store_host.TABLES lacks: register them on main first")

    def test_both_sims_compress_the_same_tables(self):
        lists = {}
        for sim, path in (("strims-sim", catalog.DOC_FILES[0]), ("ppspp", catalog.DOC_FILES[1])):
            m = re.search(r"\bZSTD: \[&str; \d+\] = \[(.*?)\];", path.read_text(), re.S)
            self.assertIsNotNone(m, f"{path}: no ZSTD list")
            lists[sim] = sorted(re.findall(r'"([a-z_]+)"', m.group(1)))
        self.assertEqual(lists["strims-sim"], lists["ppspp"], "the sims' ZSTD lists")
        self.assertEqual(sorted(set(lists["ppspp"]) - set(store_host.TABLES)), [])

    def test_the_knobs_are_the_sims_docs(self):
        self.assertEqual(sorted(set(query.TABLE_KNOBS) - set(store_host.TABLES)), [])
        for sim, prefix in (("ppspp", "SIM_"), ("strims-sim", "STRIMS_SIM_")):
            for t, (grain, _) in catalog.doc_tables()[sim].items():
                with self.subTest(sim=sim, table=t):
                    self.assertEqual(sorted(re.findall(rf"`{prefix}([A-Z_]+)=1`", grain)),
                                     sorted(query.TABLE_KNOBS.get(t, ())), f"query.TABLE_KNOBS against {grain!r}")


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
