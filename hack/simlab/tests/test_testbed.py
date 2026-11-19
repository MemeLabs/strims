#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.testbed: testbed runs as their twins' cells, and the tier; simlab.bench:
their shaper replays.

  Cells    a run's knobs name its tier cell's testbed variant
           (`<cell>@<tree>`) but for what a testbed run always sets
           (TESTBED_FIXED, tbf links); one with another TESTBED_ setting or
           of no cell names its knobs; each tier cell's run.sh settings,
           through knobs_of, name that cell
  Joins    an arrivals profile's joins hold its bounds and the twin's
           arrival order (soonest first), to the ms, anywhere in the
           runner's tick, one seed's alike; the committed tier's spread
           over the tick
  Tier     the tier's cells are hostable, honest, of the robust and
           browsers tiers; hack/sim-farm/matrices/testbed-tier is what
           tier_matrices writes
  Folds    an ingest folds a run's tables from a dir of its own and leaves
           the run dirs as they were (a tables/ their maker wrote, a
           read-only dir); tables() writes inside no run dir and into no
           dir that is there already
  Replays  a run's two shaper replays (simlab.bench), of its every offer
           and of what its link carried, are jobs of its source's cell,
           numbered as an ingest numbers its runs, keyed by their code,
           their source and their offers, and never written over a source
           or into the run dirs
  ShaperFit  scoring.sql's shaper_fit_of over a fixture: the drops' rules
           read the offered replay, the departures' the carried one, a
           link with slots its delays' distance; a source with two
           replays of a kind fails
  LinkOpens  scoring.sql's link_opens_of, first_checks_of and
           link_open_fit_of over a fixture: each joiner's links in ms after
           its join, both ends' opens and the first HAVE, outside the
           join's span or with a later joiner left out; its first parent
           check's lags from the forecast as the scheduler moves them, the
           first joiner's left out; the bench's rule, a model's median
           within the testbed's repeat spread and a floor

  python3 hack/simlab/tests/test_testbed.py
"""

import json
import os
import random
import re
import subprocess
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

from simlab import suite, testbed  # noqa: E402


class Cells(unittest.TestCase):
    def test_a_runs_knobs_name_its_tier_cells_testbed_variant(self):
        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        env = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "STRIMS_SIM_CLOCK_SKEW": "off",
               "STRIMS_SIM_CAL_SEED_BURST": "3028", "STRIMS_SIM_CAL_VIEWER_BURST": "3028",
               "TESTBED_QDISC": "clsact>ifb-netem>root-class", "STRIMS_SIM_CAL_SEED_LINK": "tbf",
               "STRIMS_SIM_CAL_VIEWER_LINK": "tbf"}
        for k in testbed.JOB_KNOBS:
            env.pop(k, None)
        got = testbed.cells_of([
            env, {**env, "STRIMS_SIM_CAL_LINKS": "1:tbf 2:tbf"},
            # The tree that shaped it is in the name: two trees never pair.
            {**env, "TESTBED_QDISC": "netem>tbf"},
            # Another link class, a setting no twin knob takes, another cell.
            {**env, "STRIMS_SIM_CAL_VIEWER_LINK": "cable,slot=2-4,aqm=fifo"},
            {**env, "TESTBED_SEED_LOSS_PCT": "1"}, {**env, "STRIMS_SIM_CAL_SECS": "61"},
            {**env, "STRIMS_SIM_SEED": "3"}])
        self.assertEqual(got[:3], ["robust:star-15-10@root-class", "robust:star-15-10@root-class",
                                   "robust:star-15-10@tbf"])
        self.assertTrue(all(x.startswith("tb-") for x in got[3:6]), got)
        self.assertEqual(len(set(got[3:6])), 3)
        # A job key is no knob of its cell's.
        self.assertEqual(got[6], got[0])

    def test_a_twin_stands_for_one_cells_knobs_and_no_testbed_setting(self):
        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        env = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "TESTBED_QDISC": "clsact>ifb-netem>root-class"}
        cell = "robust:star-15-10@root-class"
        self.assertEqual(testbed.twin_cells([(cell, env), (cell, env)]), {cell: env})
        with self.assertRaisesRegex(ValueError, r"two knobs \(STRIMS_SIM_CAL_SEED_BURST\)"):
            testbed.twin_cells([(cell, env), (cell, {**env, "STRIMS_SIM_CAL_SEED_BURST": "1514"})])
        for odd in ("TESTBED_SEED_LOSS_PCT", "TESTBED_LINKS", "TESTBED_STAMPS"):
            with self.assertRaisesRegex(ValueError, f"{odd}, which no twin knob takes"):
                testbed.twin_cells([("tb-x", {**env, odd: "1"})])

    def test_an_ingest_doesnt_overwrite_a_source(self):
        with tempfile.TemporaryDirectory() as d, mock.patch.dict(os.environ, {"SIM_DB_ROOT": d}):
            (Path(d) / "cache" / "testbed" / "old").mkdir(parents=True)
            with self.assertRaisesRegex(FileExistsError, "testbed/old exists"):
                testbed.ingest([], "old")
            testbed.refuse_existing("testbed/old", replace=True)

    def test_the_tiers_runs_name_their_cells_through_knobs_of(self):
        # run.sh's settings for each tier cell (run_env), read by
        # testbed_tables --knobs (strims-sim's knobs_of) as run.sh's out
        # dir holds them, its nodes' tc logs in it, name that cell's testbed
        # variant where run.sh stamped them from its clock (STAMPS=clock);
        # its date's stamps (STAMPS=date, or none named), or a clock's that
        # go back, key the run apart. Each says its nodes made their links
        # at their joins all at once (JOIN_LINKS=together), its
        # containers' neighbours were set before they started
        # (NEIGHBOURS=static), and its nodes ran their swarm through their
        # transfers (SWARMS=transfers).
        if not testbed.BINARY.exists():
            self.skipTest(f"no {testbed.BINARY}: cargo build --release -p strims-sim --example testbed_tables")
        matrices = HACK / "sim-farm" / "matrices"
        tc = ("{t} qdisc tbf 10: root refcnt 2 rate 10Mbit burst 3028b lat 99ms  Sent 0 bytes 0 pkt (dropped 0, "
              "overlimits 0 requeues 0)  backlog 0b 0p requeues 0\n")
        cases = {"clock": ("STAMPS=clock\n", [1000, 1100, 1200]), "date": ("STAMPS=date\n", [1000, 1100, 1200]),
                 "none": ("", [1000, 1100, 1200]), "back": ("STAMPS=clock\n", [1000, 1481, 1100])}
        got = {c: [] for c in cases}
        want = []
        with tempfile.TemporaryDirectory() as d:
            for kind, cell in testbed.tier_cells():
                settings, knobs, topology = testbed.run_env(kind, cell)
                viewers = int(settings["VIEWERS"])
                want.append(f"{'robust' if kind == 'native' else 'browsers'}:{cell['name']}@root-class")
                for case, (stamps, ts) in cases.items():
                    out = Path(d) / f"{kind}-{cell['name']}-{case}"
                    out.mkdir()
                    (out / "settings").write_text("".join(f"{k}={v}\n" for k, v in settings.items())
                                                  + "QDISC=clsact>ifb-netem>root-class\nJOIN_LINKS=together\n"
                                                  + "NEIGHBOURS=static\nSWARMS=transfers\n"
                                                  + stamps)
                    links = ((matrices / "topologies" / topology).read_text() if topology else
                             "".join(f"link 0 {v}\n" for v in range(1, viewers + 1)))
                    (out / "topology").write_text(links)
                    up = lambda i: settings["SEED_UP_MBIT"] if i == 0 else settings.get("VIEWER_UP_MBIT", "20")
                    (out / "links").write_text("".join(f"node {i} tbf {up(i)} 10 100\n" for i in range(viewers + 1)))
                    for i in range(viewers + 1):
                        (out / f"tc-{i}.log").write_text("".join(tc.format(t=1794800000000 + t) for t in ts))
                    p = subprocess.run([str(testbed.BINARY), "--knobs", str(out)], capture_output=True, text=True)
                    self.assertEqual(p.returncode, 0, f"{kind} {cell['name']} {case}: {p.stderr}")
                    got[case].append({**json.loads(p.stdout), **knobs})
        self.assertEqual(testbed.cells_of(got["clock"]), want)
        for case in ("date", "none", "back"):
            with self.subTest(case=case):
                stamps = {e.get("TESTBED_STAMPS") for e in got[case]}
                self.assertEqual(stamps, {"back" if case == "back" else "date"})
                self.assertTrue(all(c.startswith("tb-") for c in testbed.cells_of(got[case])))


class Joins(unittest.TestCase):
    def parse(self, joins):
        return [(int(i), float(t)) for i, t in (p.split(":") for p in joins.split())]

    def test_a_trickle_joins_within_the_streams_first_third_soonest_first(self):
        js = self.parse(testbed.joins_of("trickle", 15, 60, random.Random(1)))
        self.assertEqual([i for i, _ in js], list(range(1, 16)))
        ts = [t for _, t in js]
        self.assertEqual(ts, sorted(ts))
        self.assertTrue(all(0 <= t <= 20 for t in ts))

    def test_an_events_warm_share_joins_before_its_burst(self):
        js = self.parse(testbed.joins_of("event:20:20:10", 15, 60, random.Random(2)))
        ts = [t for _, t in js]
        self.assertEqual(sum(t < 20 for t in ts), 3)
        self.assertTrue(all(t <= 30 for t in ts))

    def test_joins_are_drawn_to_the_ms_anywhere_in_the_tick_and_alike_from_one_seed(self):
        # The runner ticks every 100 ms from a node's first channel: joins
        # at one phase of it would lock every node's tick to every later
        # join, so a draw's phases spread over it, as the twin's do.
        for arrivals in ("trickle", "event:20:20:10"):
            with self.subTest(arrivals=arrivals):
                joins = testbed.joins_of(arrivals, 31, 180, random.Random("x"))
                self.assertEqual(joins, testbed.joins_of(arrivals, 31, 180, random.Random("x")))
                times = [t for _, t in (p.split(":") for p in joins.split())]
                self.assertTrue(all(re.fullmatch(r"\d+\.\d{3}", t) for t in times), times)
                phases = {round(float(t) * 1000) % 100 for t in times}
                self.assertGreater(len(phases), 20, sorted(phases))

    def test_the_committed_tiers_joins_spread_over_the_tick(self):
        # Every repeat's JOINS has joins at two phases of the tick at
        # least, so strims-sim's testbed module keys none of its runs
        # apart (TESTBED_JOINS).
        draws = [json.loads(line.split("=", 1)[1])
                 for p in (HACK / "sim-farm" / "matrices" / "testbed-tier").glob("*.toml")
                 for line in p.read_text().splitlines() if line.startswith("JOINS = ")]
        self.assertTrue(draws)
        for joins in (j for d in draws for j in d):
            phases = {round(float(p.split(":")[1]) * 1000) % 100 for p in joins.split()}
            self.assertGreater(len(phases), 1, joins)

    def test_another_profile_is_refused(self):
        with self.assertRaises(ValueError):
            testbed.joins_of("trickle:2", 3, 60, random.Random(1))


class Measured(unittest.TestCase):
    def test_a_runs_recorded_starts_neither_key_it_nor_split_its_twin(self):
        base = {"STRIMS_SIM_CAL_VIEWERS": "15", "STRIMS_SIM_CAL_COMPETE": "10:20", "TESTBED_QDISC": "a>root-class"}
        runs = [{**base, "STRIMS_SIM_CAL_COMPETE_MS": "10201:20"}, {**base, "STRIMS_SIM_CAL_COMPETE_MS": "10164:20"}]
        a, b = testbed.cells_of(runs)
        self.assertEqual(a, b)
        twin = testbed.twin_cells([("c", runs[0]), ("c", runs[1])])
        self.assertEqual(twin["c"]["STRIMS_SIM_CAL_COMPETE_MS"], "10182:20")
        runs.append({**base, "STRIMS_SIM_CAL_COMPETE_MS": "10170:20"})
        self.assertEqual(testbed.twin_cells([("c", r) for r in runs])["c"]["STRIMS_SIM_CAL_COMPETE_MS"], "10170:20")
        with self.assertRaisesRegex(ValueError, "two sets of uploads"):
            testbed.twin_cells([("c", runs[0]), ("c", {**base, "STRIMS_SIM_CAL_COMPETE_MS": "10201:30"})])


# A stand-in for strims-sim's scenarios binary: its one row names the
# packet log it ran with, and it writes a packet a host of the log's
# numbers into its STRIMS_SIM_OUT, as calibration_star's packets table.
FAKE_SCENARIOS = """\
import json, os
from pathlib import Path
out = Path(os.environ["STRIMS_SIM_OUT"])
out.mkdir(parents=True, exist_ok=True)
log = os.environ.get("STRIMS_SIM_PACKET_LOG")
if log and log not in ("seed", "all", "viewers"):
    (out / "packets.schema.json").write_text(json.dumps({"columns": [{"name": "node", "type": "BIGINT"},
                                                                     {"name": "n", "type": "BIGINT"}]}))
    (out / "packets.csv").write_text("node,n\\n" + "".join(f"{h},0\\n" for h in sorted(set(log.split(",")))))
print("ROW " + json.dumps({"packet_log": log}))
"""


class Twins(unittest.TestCase):
    """twins over a side of the catalog (a fixture of its jobs and packets
    tables, testbed_side's and packet_hosts_of's macros made over them as
    a catalog's reader makes them): source testbed/x's runs of cell a hold
    the packets of nodes 0 and 1, and of 0, 1 and 2 (PCAP=1's captures),
    cell b's none, cell c's node 1's alone; another source's run of cell a
    holds node 5's. A lake without packets (main's, before any testbed
    run's capture) has no packets table at all."""

    JOBS = [("ta1", "testbed/x", "a", 1), ("ta2", "testbed/x", "a", 2), ("tb1", "testbed/x", "b", 1),
            ("tc1", "testbed/x", "c", 1), ("to1", "testbed/other", "a", 1)]
    PACKETS = {"ta1": [0, 1], "ta2": [0, 1, 2], "tc1": [1], "to1": [5]}

    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.d = Path(tmp.name)
        p = mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.d / "sim-db")})
        p.start()
        self.addCleanup(p.stop)
        self.fake = self.d / "scenarios"
        self.fake.write_text(f"#!{sys.executable}\n{FAKE_SCENARIOS}")
        self.fake.chmod(0o755)

    def lake(self, packets=True):
        """catalog.lake over the fixture, with a packets table or none, its
        macros made as a reader of such a catalog makes them
        (catalog.needed: packet_hosts_of's fallback where it lacks
        packets)."""
        import duckdb

        from simlab import catalog

        conn = duckdb.connect()
        self.addCleanup(conn.close)
        conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, sim VARCHAR, stored_cell VARCHAR, "
                     "stored_arm VARCHAR, seed BIGINT, env MAP(VARCHAR, VARCHAR))")
        for k, src, cell, seed in self.JOBS:
            env = {"STRIMS_SIM_CAL_VIEWERS": {"a": "2", "b": "3", "c": "1"}[cell], "STRIMS_SIM_TCP": "1",
                   "TESTBED_QDISC": "clsact>ifb-netem>root-class", "STRIMS_SIM_SEED": str(seed)}
            conn.execute("INSERT INTO jobs VALUES (?, ?, 'testbed', ?, 'testbed', ?, MAP(?::VARCHAR[], ?::VARCHAR[]))",
                         [k, src, cell, seed, list(env), list(env.values())])
        if packets:
            conn.execute("CREATE TABLE packets (k VARCHAR, src VARCHAR, node BIGINT, n BIGINT)")
            srcs = {k: src for k, src, _, _ in self.JOBS}
            for k, nodes in self.PACKETS.items():
                for node in nodes:
                    conn.executemany("INSERT INTO packets VALUES (?, ?, ?, ?)",
                                     [(k, srcs[k], node, n) for n in range(3)])
        have = {"packets": {"k", "src", "node", "n"}} if packets else {}
        text = catalog.needed(catalog.scoring_text(catalog.scoring_consts()), have)
        for m in catalog.MACRO.finditer(text):
            if m.group(2) in ("testbed_side", "packet_hosts_of"):
                conn.execute(text[m.start():catalog.STATEMENT_END.search(text, m.end()).end()])

        def lake(sql, **kw):
            cur = conn.execute(sql)
            names = [d[0] for d in cur.description]
            return [dict(zip(names, r)) for r in cur.fetchall()]

        p = mock.patch.object(catalog, "lake", lake)
        p.start()
        self.addCleanup(p.stop)

    def logs(self, jobs):
        """Each twin's packet log, as its knobs give it and as it ran."""
        return {(j["cell"], j["seed"]): (j["env"].get("STRIMS_SIM_PACKET_LOG"), j["row"]["packet_log"]) for j in jobs}

    def test_a_pcap_runs_twin_logs_its_seeds_packets(self):
        # By default each cell's twin logs the seed's packets where its
        # runs hold them, the side's alone: cell c's hold host 1's alone,
        # cell b's none.
        self.lake()
        cells = testbed.cells_in("testbed/x")
        self.assertEqual({c: e.get("STRIMS_SIM_PACKET_LOG") for c, e in cells.items()}, {"a": "0", "b": None, "c": None})
        src, jobs = testbed.twins("testbed/x", self.fake, "x-twin", seeds=2)
        self.assertEqual(src, "testbed/x-twin")
        self.assertEqual(self.logs(jobs), {("a", 1): ("0", "0"), ("a", 2): ("0", "0"), ("b", 1): (None, None),
                                           ("b", 2): (None, None), ("c", 1): (None, None), ("c", 2): (None, None)})
        # The twins' packets in the cache: the seed's in each of cell a's
        # two runs.
        manifest = json.loads((self.d / "sim-db" / "cache" / "testbed" / "x-twin" / "manifest.json").read_text())
        self.assertEqual(manifest["tables"]["packets"]["rows"], 2)

    def test_a_twin_logs_the_hosts_it_is_asked_for_that_its_side_holds(self):
        self.lake()
        _, jobs = testbed.twins("testbed/x", self.fake, "x-twin", seeds=1, packet_hosts="all")
        self.assertEqual(self.logs(jobs), {("a", 1): ("0,1,2", "0,1,2"), ("b", 1): (None, None),
                                           ("c", 1): ("1,1", "1,1")})
        _, jobs = testbed.twins("testbed/x", self.fake, "x-twin", seeds=1, packet_hosts=[2, 5], replace=True)
        self.assertEqual(self.logs(jobs), {("a", 1): ("2", "2"), ("b", 1): (None, None), ("c", 1): (None, None)})
        self.assertEqual(testbed.logged_hosts([0, 1, 2], "seed"), [0])
        self.assertEqual(testbed.logged_hosts([1, 2], "seed"), [])
        self.assertEqual(testbed.logged_hosts([0, 1, 2], ["2", 0]), [0, 2])

    def test_a_lake_without_packets_twins_without_a_log(self):
        # No source holds packets: the catalog has no such table, and the
        # twins log none; a side of no runs says so.
        self.lake(packets=False)
        _, jobs = testbed.twins("testbed/x", self.fake, "x-twin", seeds=1, packet_hosts="all")
        self.assertEqual(self.logs(jobs), {("a", 1): (None, None), ("b", 1): (None, None), ("c", 1): (None, None)})
        with self.assertRaisesRegex(ValueError, r"no testbed runs in testbed/none \(one, one\)"):
            testbed.twins("testbed/none", self.fake, "none-twin")

    def test_a_setting_the_caller_gives_stands(self):
        self.lake()
        _, jobs = testbed.twins("testbed/x", self.fake, "x-twin", seeds=1, env={"STRIMS_SIM_PACKET_LOG": "seed"})
        self.assertEqual({j["cell"]: j["row"]["packet_log"] for j in jobs}, {"a": "seed", "b": "seed", "c": "seed"})

    def test_a_checkout_without_target_twins_under_the_caches_tmp(self):
        # A fresh worktree has no target/: the twins run in a dir of their
        # own under the cache's tmp/ (work_dir), removed after, and make
        # nothing in the checkout.
        self.lake()
        root = self.d / "checkout"
        root.mkdir()
        tmp = self.d / "sim-db" / "tmp"
        with mock.patch.object(testbed, "ROOT", root):
            src, jobs = testbed.twins("testbed/x", self.fake, "x-twin", seeds=1)
        self.assertEqual(src, "testbed/x-twin")
        self.assertEqual({(j["cell"], j["seed"]) for j in jobs}, {("a", 1), ("b", 1), ("c", 1)})
        self.assertEqual({Path(j["dir"]).parent.parent for j in jobs}, {tmp})
        self.assertEqual(list(root.iterdir()), [])
        self.assertEqual(list(tmp.iterdir()), [])

    def test_the_packet_logs_value_names_the_hosts(self):
        self.assertEqual(testbed.packet_log([2, 0, 1, 0]), "0,1,2")
        self.assertEqual(testbed.packet_log([3]), "3")
        # A lone 1 reads as every host (`all`'s `1`): host 1 twice.
        self.assertEqual(testbed.packet_log([1]), "1,1")


class Tier(unittest.TestCase):
    def test_the_binary_is_where_cargo_writes_it(self):
        # $CARGO_TARGET_DIR, relative to the checkout or absolute, else
        # the checkout's target/, as the engine set's build writes it.
        def binary(env):
            env = {k: v for k, v in os.environ.items() if k != "CARGO_TARGET_DIR"} | env
            return subprocess.run([sys.executable, "-c", "from simlab import testbed; print(testbed.BINARY)"],
                                  cwd=HACK, env=env, capture_output=True, text=True, check=True).stdout.strip()

        root = HACK.parent
        self.assertEqual(binary({}), str(root / "target/release/examples/testbed_tables"))
        self.assertEqual(binary({"CARGO_TARGET_DIR": "/x/t"}), "/x/t/release/examples/testbed_tables")
        self.assertEqual(binary({"CARGO_TARGET_DIR": "t2"}), str(root / "t2/release/examples/testbed_tables"))

    def test_the_tiers_cells_are_hostable_and_honest(self):
        cells = testbed.tier_cells()
        self.assertTrue(cells)
        for kind, c in cells:
            with self.subTest(cell=c["name"], kind=kind):
                env = c["env"]
                self.assertEqual(c["test"], "calibration_star")
                self.assertEqual(env["STRIMS_SIM_HOSTILE"], "off")
                self.assertFalse(any(k in env for k in testbed.UNHOSTED))
                self.assertLessEqual(int(env["STRIMS_SIM_CAL_VIEWERS"]), testbed.MAX_VIEWERS[kind])
                self.assertEqual("STRIMS_SIM_RTC" in env, kind == "browsers")

    def test_the_committed_tier_is_tier_matrices(self):
        committed = HACK / "sim-farm" / "matrices" / "testbed-tier"
        with tempfile.TemporaryDirectory() as d:
            out = Path(d) / "testbed-tier"
            paths = testbed.tier_matrices(out, repeats=2, ref="main")
            self.assertEqual(sorted(p.name for p in paths), sorted(p.name for p in committed.glob("*.toml")))
            # A topology's path is relative to its matrix's dir: compare
            # its file.
            def plain(text):
                return [Path(line.split('"')[1]).name if line.startswith("TOPOLOGY = ") else line
                        for line in text.splitlines()]

            for p in paths:
                with self.subTest(matrix=p.name):
                    self.assertEqual(plain((committed / p.name).read_text()), plain(p.read_text()))


def snapshot(root):
    """Every dir and file under `root`: its mode and mtime, and a file's
    bytes' digest."""
    import hashlib

    out = {}
    for p in sorted(Path(root).rglob("*")):
        st = p.lstat()
        out[str(p.relative_to(root))] = (st.st_mode, st.st_mtime_ns,
                                          hashlib.sha256(p.read_bytes()).hexdigest() if p.is_file() else None)
    return out


def read_only(root):
    """`root` and what it holds made read-only; undone by the returned
    function."""
    paths = [Path(root), *Path(root).rglob("*")]
    modes = {p: p.stat().st_mode for p in paths}
    for p in reversed(paths):
        p.chmod(modes[p] & ~0o222)
    return lambda: [p.chmod(m) for p, m in modes.items()]


# A stand-in for strims-sim's testbed_tables: writes into its tables dir
# (made where it isn't there, as the binary's create_dir_all does) a
# `job.json`, its key `tb-` and the run dir's name, its env the knobs of
# the run dir's `knobs.json` and its `--knob`s, and one packet.
FAKE_TABLES = """\
import json, sys
from pathlib import Path
run, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True, exist_ok=True)
env = {**json.loads((run / "knobs.json").read_text()), **dict(a.split("=", 1) for a in sys.argv[4::2])}
(out / "job.json").write_text(json.dumps({"k": "tb-" + run.name, "sim": "testbed", "test": "calibration_star",
                                          "env": env, "row": {}}))
(out / "packets.schema.json").write_text(json.dumps({"columns": [{"name": "node", "type": "BIGINT"},
                                                                 {"name": "n", "type": "BIGINT"}]}))
(out / "packets.csv").write_text("node,n\\n0,0\\n")
"""


class Folds(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        self.d = Path(tmp.name)
        env = mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.d / "lake")})
        env.start()
        self.addCleanup(env.stop)
        self.fake = self.d / "testbed_tables"
        self.fake.write_text(f"#!{sys.executable}\n{FAKE_TABLES}")
        self.fake.chmod(0o755)
        # Two runs of one cell's knobs, as run.sh leaves them; the first
        # holds tables its maker wrote (another session's build, say).
        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        knobs = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "STRIMS_SIM_CLOCK_SKEW": "off",
                 "STRIMS_SIM_CAL_SEED_BURST": "3028", "STRIMS_SIM_CAL_VIEWER_BURST": "3028",
                 "TESTBED_QDISC": "clsact>ifb-netem>root-class"}
        self.runs = []
        for name in ("a", "b"):
            run = self.d / "runs" / name
            run.mkdir(parents=True)
            (run / "knobs.json").write_text(json.dumps(knobs))
            (run / "node-0.jsonl").write_text('{"rec": 1}\n')
            self.runs.append(run)
        theirs = self.runs[0] / "tables"
        theirs.mkdir()
        (theirs / "job.json").write_text('{"k": "tb-theirs"}\n')
        (theirs / "seconds.csv").write_text("sec\n0\n")

    def test_a_fold_leaves_its_run_dirs_as_they_were(self):
        before = snapshot(self.d / "runs")
        src, jobs = testbed.ingest(self.runs, "t", binary=self.fake, knobs={"STRIMS_SIM_ARRIVALS": "trickle"})
        self.assertEqual(snapshot(self.d / "runs"), before)
        self.assertEqual(src, "testbed/t")
        # Each a job of its knobs' cell (the --knob's with them), numbered
        # in it, `run` the dir it was read from.
        self.assertEqual([(j["k"], j["run"], j["seed"]) for j in jobs],
                         [("tb-a", str(self.runs[0]), 1), ("tb-b", str(self.runs[1]), 2)])
        self.assertEqual(jobs[0]["cell"], jobs[1]["cell"])
        self.assertTrue(all("dir" not in j and j["env"]["STRIMS_SIM_ARRIVALS"] == "trickle" for j in jobs))
        manifest = json.loads((self.d / "lake" / "cache" / "testbed" / "t" / "manifest.json").read_text())
        self.assertEqual((manifest["keys"], manifest["tables"]["packets"]["rows"]), (["tb-a", "tb-b"], 2))
        # The fold's own dirs went with it.
        self.assertEqual(list((self.d / "lake" / "tmp").iterdir()), [])

    def test_a_fold_reads_a_read_only_run_dir(self):
        self.addCleanup(read_only(self.d / "runs"))
        before = snapshot(self.d / "runs")
        _, jobs = testbed.ingest(self.runs, "t", binary=self.fake)
        self.assertEqual([(j["k"], j["cell"]) for j in jobs], [("tb-a", "robust:star-15-10@root-class"),
                                                               ("tb-b", "robust:star-15-10@root-class")])
        self.assertEqual(snapshot(self.d / "runs"), before)

    def test_tables_go_into_a_new_dir_outside_the_run(self):
        run = self.runs[1]
        before = snapshot(self.d / "runs")
        # Inside the run dir, or a dir that is there (another run's, one
        # holding the run).
        for out, err in ((run / "tables", ValueError), (run / "x" / "tables", ValueError),
                         (self.runs[0], FileExistsError), (self.d, FileExistsError)):
            with self.subTest(out=out), self.assertRaises(err):
                testbed.tables(run, out, binary=self.fake)
        self.assertEqual(snapshot(self.d / "runs"), before)
        out = testbed.tables(run, self.d / "out" / "b", binary=self.fake)
        self.assertEqual(json.loads((out / "job.json").read_text())["k"], "tb-b")
        self.assertEqual(snapshot(self.d / "runs"), before)


# A stand-in for strims-sim's shaper_replay: writes `<tables dir>/job.json`
# as it does, its env the knobs of the run dir's `knobs.json` and its key
# the dir's name, and one packet a node of its `--node`s.
FAKE_REPLAY = """\
import json, sys
from pathlib import Path
run, out = Path(sys.argv[1]), Path(sys.argv[2])
out.mkdir(parents=True)
args = sys.argv[3:]
nodes = [args[i + 1] for i, a in enumerate(args) if a == "--node"] or ["0"]
offers = args[args.index("--offers") + 1]
env = {**json.loads((run / "knobs.json").read_text()), "SOURCE_K": run.name, "REPLAY_NODES": ",".join(nodes),
       "REPLAY_OFFERS": offers}
(out / "job.json").write_text(json.dumps({"k": "rp-" + run.name, "sim": "strims-sim", "test": "shaper_replay",
                                          "env": env, "row": {}}))
(out / "packets.schema.json").write_text(json.dumps({"columns": [{"name": "node", "type": "BIGINT"},
                                                                 {"name": "n", "type": "BIGINT"}]}))
(out / "packets.csv").write_text("node,n\\n" + "".join(f"{n},0\\n" for n in nodes))
"""


class Replays(unittest.TestCase):
    def test_a_replay_is_a_job_of_its_sources_cell_keyed_by_its_code(self):
        from simlab import bench

        c = next(c for c in suite.TIERS["robust"] if c["name"] == "star-15-10")
        knobs = {**c["env"], "STRIMS_SIM_HOSTILE": "off", "STRIMS_SIM_CLOCK_SKEW": "off",
                 "STRIMS_SIM_CAL_SEED_BURST": "3028", "STRIMS_SIM_CAL_VIEWER_BURST": "3028",
                 "TESTBED_QDISC": "clsact>ifb-netem>root-class"}
        with tempfile.TemporaryDirectory() as d, mock.patch.dict(os.environ, {"SIM_DB_ROOT": d}):
            d = Path(d)
            fake = d / "shaper_replay"
            fake.write_text(f"#!{sys.executable}\n{FAKE_REPLAY}")
            fake.chmod(0o755)
            runs = []
            for name, k in (("tb-a", knobs), ("tb-b", knobs), ("tb-c", {**knobs, "TESTBED_SEED_LOSS_PCT": "1"})):
                (d / name).mkdir()
                (d / name / "knobs.json").write_text(json.dumps(k))
                runs.append(d / name)
            # A replay its maker wrote into the run dir stays theirs.
            (d / "tb-a" / "replay").mkdir()
            (d / "tb-a" / "replay" / "job.json").write_text('{"k": "rp-theirs"}\n')
            before = {r.name: snapshot(r) for r in runs}
            src, jobs = bench.replays(runs, "r", binary=fake, nodes=[0, 3])
            self.assertEqual(src, "testbed/r")
            self.assertEqual([(j["env"]["SOURCE_K"], j["env"]["REPLAY_OFFERS"]) for j in jobs],
                             [(r, o) for r in ("tb-a", "tb-b", "tb-c") for o in ("offered", "carried")])
            # Its source's cell, numbered as an ingest numbers its runs; the
            # arms replay and replay:carried; its key the code's, its
            # source's and its offers'.
            self.assertEqual([(j["cell"], j["seed"], j["arm"]) for j in jobs][:4],
                             [("robust:star-15-10@root-class", 1, "replay"),
                              ("robust:star-15-10@root-class", 1, "replay:carried"),
                              ("robust:star-15-10@root-class", 2, "replay"),
                              ("robust:star-15-10@root-class", 2, "replay:carried")])
            self.assertTrue(jobs[4]["cell"].startswith("tb-") and jobs[4]["cell"] == jobs[5]["cell"])
            self.assertTrue(all(j["k"].startswith("rp-") and j["k"] != "rp-" + j["env"]["SOURCE_K"] for j in jobs))
            self.assertEqual(len({j["k"] for j in jobs}), 6)
            self.assertEqual({r.name: snapshot(r) for r in runs}, before)
            self.assertEqual([j["run"] for j in jobs], [str(r) for r in runs for _ in bench.OFFERS])
            manifest = json.loads((d / "cache" / "testbed" / "r" / "manifest.json").read_text())
            self.assertEqual(manifest["tables"]["packets"]["rows"], 12)
            self.assertEqual(list((d / "tmp").iterdir()), [])
            # Another binary keys apart, and the source isn't written over.
            fake.write_text(fake.read_text() + "\n# another build\n")
            with self.assertRaisesRegex(FileExistsError, "testbed/r exists"):
                bench.replays(runs, "r", binary=fake)
            _, again = bench.replays(runs[:1], "r", binary=fake, nodes=[0, 3], replace=True)
            self.assertNotEqual(again[0]["k"], jobs[0]["k"])



class ShaperFit(unittest.TestCase):
    """shaper_fit_of over a fixture: a source run (k tb) and its two
    replays, `ro` of every offer and `rc` of what the testbed carried.

    Node 0 (tbf, 1.2 ms a frame): the testbed drops packet 2 and the
    offered replay packet 5, so the offered replay's later departures
    trail the testbed's by a frame, where the carried replay's, fed only
    what both queues held, are within 100 µs. Node 1 (cable, slots): the
    carried replay's delays the testbed's. Node 2 (tbf): the carried
    replay didn't send a packet the testbed carried."""

    PACKETS = [  # k, node, n, peer, t_us, sent_us, lost
        ("tb", 0, 0, 1, 0, 1200, None), ("tb", 0, 1, 2, 0, 2400, None), ("tb", 0, 2, 1, 0, None, "queue"),
        ("tb", 0, 3, 2, 0, 3600, None), ("tb", 0, 4, 1, 0, 4800, None), ("tb", 0, 5, 2, 0, 6000, None),
        ("ro", 0, 0, 1, 0, 1200, None), ("ro", 0, 1, 2, 0, 2400, None), ("ro", 0, 2, 1, 0, 3600, None),
        ("ro", 0, 3, 2, 0, 4800, None), ("ro", 0, 4, 1, 0, 6000, None), ("ro", 0, 5, 2, 0, None, "queue"),
        ("rc", 0, 0, 1, 0, 1210, None), ("rc", 0, 1, 2, 0, 2420, None), ("rc", 0, 3, 2, 0, 3650, None),
        ("rc", 0, 4, 1, 0, 4800, None), ("rc", 0, 5, 2, 0, 6000, None),
        ("tb", 1, 0, 0, 0, 2000, None), ("tb", 1, 1, 0, 1000, 3000, None), ("tb", 1, 2, 0, 2000, 4000, None),
        ("ro", 1, 0, 0, 0, 2000, None), ("ro", 1, 1, 0, 1000, 3000, None), ("ro", 1, 2, 0, 2000, 4000, None),
        ("rc", 1, 0, 0, 0, 2000, None), ("rc", 1, 1, 0, 1000, 3000, None), ("rc", 1, 2, 0, 2000, 4000, None),
        ("tb", 2, 0, 0, 0, 1200, None), ("tb", 2, 1, 0, 0, 2400, None),
        ("ro", 2, 0, 0, 0, 1200, None), ("ro", 2, 1, 0, 0, 2400, None),
        ("rc", 2, 0, 0, 0, 1200, None), ("rc", 2, 1, 0, 0, None, "queue"),
    ]
    HOSTS = [(0, "seed", "tbf"), (1, "viewer", "cable,slot=2"), (2, "viewer", "tbf")]
    SOURCE = "[{'cell': 'c', 'seed': 1, 'k': 'tb', 'src': 'testbed/s'}]"

    @classmethod
    def setUpClass(cls):
        import duckdb

        from simlab import catalog

        consts = catalog.scoring_consts()
        text = catalog.CONST.sub(lambda m: str(consts[m.group(1)]), catalog.SCORING.read_text())
        conn = duckdb.connect()
        for m in re.finditer(r"^CREATE OR REPLACE MACRO sketch_\w+.*?;$", text, re.M | re.S):
            conn.execute(m.group(0))
        conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, env MAP(VARCHAR, VARCHAR))")
        nodes = {"REPLAY_NODES": "0,1,2"}
        for k, src, env in (("tb", "testbed/s", {}),
                            ("ro", "testbed/r", {"SOURCE_K": "tb", "REPLAY_OFFERS": "offered", **nodes}),
                            ("rc", "testbed/r", {"SOURCE_K": "tb", "REPLAY_OFFERS": "carried", **nodes}),
                            ("ro2", "testbed/r2", {"SOURCE_K": "tb", "REPLAY_OFFERS": "offered", **nodes}),
                            ("tw", "testbed/tw", {"STRIMS_SIM_CAL_VIEWERS": "2"})):
            conn.execute("INSERT INTO jobs VALUES (?, ?, MAP(?::VARCHAR[], ?::VARCHAR[]))",
                         [k, src, list(env), list(env.values())])
        srcs = {"tb": "testbed/s", "ro": "testbed/r", "rc": "testbed/r"}
        conn.execute("CREATE TABLE hosts (k VARCHAR, src VARCHAR, node BIGINT, role VARCHAR, link_spec VARCHAR)")
        conn.executemany("INSERT INTO hosts VALUES ('tb', 'testbed/s', ?, ?, ?)", cls.HOSTS)
        conn.execute("CREATE TABLE packets (k VARCHAR, src VARCHAR, node BIGINT, n BIGINT, peer BIGINT, t_us BIGINT, "
                     "sent_us BIGINT, lost VARCHAR)")
        conn.executemany("INSERT INTO packets VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                         [(k, srcs[k], *rest) for k, *rest in cls.PACKETS]
                         + [("ro2", "testbed/r2", *rest) for k, *rest in cls.PACKETS if k == "ro"])
        (macro,) = catalog.scoring_statements(["shaper_fit_of"], {"packets", "hosts", "jobs.env"})
        conn.execute(macro)
        cls.conn = conn

    def fit(self, replays, per="node"):
        jobs = ", ".join(f"{{'cell': 'c', 'seed': 1, 'k': '{k}', 'src': NULL}}" for k in replays)
        rows = self.conn.execute(f"SELECT node, peer, measure, value, bound, pass FROM shaper_fit_of([{jobs}], "
                                 f"{self.SOURCE}, per := '{per}')").fetchall()
        return {(node, peer, measure): (value, bound, ok) for node, peer, measure, value, bound, ok in rows}

    def test_the_drops_read_the_offered_replay_and_the_departures_the_carried(self):
        got = self.fit(["ro", "rc"])
        want0 = {"packets": 6, "carried": 5, "carried_both": 5, "drops_testbed": 1, "drops_model": 1,
                 "drops_both_count": 0, "residual_mean_us": 16, "within_100us": 1.0,
                 "drops_diff": 0, "drops_both": 0}
        self.assertEqual({m: got[(0, None, m)][0] for m in want0}, want0)
        self.assertEqual([got[(0, None, m)][2] for m in ("within_100us", "drops_diff", "drops_both", "pass")],
                         [True, True, False, False])
        self.assertNotIn((0, None, "delay_ks"), got)
        # Slots: the delays' distance, no departure or drops_both rule.
        self.assertEqual(got[(1, None, "delay_ks")], (0.0, 0.1, True))
        self.assertNotIn((1, None, "within_100us"), got)
        self.assertEqual(got[(1, None, "pass")][2], True)
        # A carried packet the carried replay didn't send counts against it.
        self.assertEqual((got[(2, None, "within_100us")][0], got[(2, None, "carried_both")][0]), (0.5, 1))
        # Per peer, a link's packets apart.
        peers = self.fit(["ro", "rc"], per="peer")
        self.assertEqual((peers[(0, 1, "packets")][0], peers[(0, 2, "packets")][0]), (3, 3))
        self.assertEqual((peers[(0, 1, "drops_testbed")][0], peers[(0, 2, "drops_model")][0]), (1, 1))

    def test_a_source_takes_one_replay_of_each_kind(self):
        import duckdb

        with self.assertRaisesRegex(duckdb.Error, "source tb has 2 offered and 1 carried replays"):
            self.fit(["ro", "rc", "ro2"])
        with self.assertRaisesRegex(duckdb.Error, "source tb has 1 offered and 0 carried replays"):
            self.fit(["ro"])
        # A source whose packets have no replay in a_jobs (a twin's jobs,
        # which replay nothing) has no model side.
        with self.assertRaisesRegex(duckdb.Error, "source tb has 0 offered and 0 carried replays"):
            self.fit(["tw"])

    def rolled_back(self, *statements, replays=("ro", "rc")):
        """fit's rows with `statements` applied, then undone."""
        self.conn.execute("BEGIN")
        try:
            for s in statements:
                self.conn.execute(s)
            return self.fit(list(replays))
        finally:
            self.conn.execute("ROLLBACK")

    def test_a_replay_without_the_packets_of_a_node_it_replayed_is_refused(self):
        import duckdb

        for k, kind in (("ro", "offered"), ("rc", "carried")):
            with self.assertRaisesRegex(duckdb.Error, f"shaper_fit_of: the {kind} replay {k} of source tb holds no "
                                                      r"packets rows of node 2, which it replayed \(REPLAY_NODES\)"):
                self.rolled_back(f"DELETE FROM packets WHERE k = '{k}' AND node = 2")
        # A node it didn't replay is none of its fit's.
        got = self.rolled_back("DELETE FROM packets WHERE k IN ('ro', 'rc') AND node = 2",
                               "UPDATE jobs SET env = map_concat(env, MAP {'REPLAY_NODES': '0,1'}) "
                               "WHERE k IN ('ro', 'rc')")
        self.assertEqual({node for node, _, _ in got}, {0, 1})
        # The carried replay of a node the testbed carried none of holds
        # none.
        got = self.rolled_back("UPDATE packets SET sent_us = NULL, lost = 'queue' WHERE k = 'tb' AND node = 2",
                               "DELETE FROM packets WHERE k = 'rc' AND node = 2")
        self.assertEqual(got[(2, None, "carried")][0], 0)


class LinkOpens(unittest.TestCase):
    """link_opens_of and first_checks_of over one run: the seed (node 0)
    from the start, viewer 1 joining at 1000 ms and viewer 2 at 2000; a
    second (c) with viewer 3 at 3000 too, whose links and forecasts reach
    outside its join's span; and the bench's rule over copies of the first
    and over runs of two joiners to the seed (u1, u2, n1)."""

    EVENTS = [(1000, "join", 1), (2000, "join", 2), (1500, "first_read", 1)]
    HAVES = [  # t_ms, node, peer, channel_open_ms
        (1160, 1, 0, 1150), (1170, 1, 0, 1150), (1145, 0, 1, 1140),
        (2350, 2, 0, 2300), (2320, 0, 2, 2310), (2210, 2, 1, 2200), (2200, 1, 2, 2190),
        (9010, 2, 1, 9000),
    ]
    FORECAST = [  # t_ms, node, peer, stream, sample_ms, mean_ms, samples
        (2350, 2, 0, 0, 100, None, 1), (2380, 2, 0, 0, 120, 100, 2), (2250, 2, 1, 0, 10, None, 1),
        (2390, 2, 0, 1, 80, None, 1), (2450, 2, 0, 1, 500, 80, 2), (1190, 1, 0, 0, 30, None, 1),
    ]
    CHECKS = [  # t_us, node, stream, best_peer, best_lag_us
        (2_400_000, 2, 0, 1, 10_000), (2_400_000, 2, 1, 0, 80_000), (2_500_000, 2, 0, 0, 5_000),
        (1_200_000, 1, 0, 0, 30_000),
    ]
    # Run c: viewer 3's channel with viewer 1 opens 30 ms before its join
    # (within the span's 50), with viewer 2 100 ms before (outside it), and
    # with the seed past the span's 4 s; viewer 1's with it opens 100 ms
    # before the join (outside) and 100 ms after.
    EVENTS_C = [(1000, "join", 1), (2000, "join", 2), (3000, "join", 3)]
    HAVES_C = [(3005, 3, 1, 2970), (3010, 3, 2, 2900), (9100, 3, 0, 9050), (2950, 1, 3, 2900),
               (3150, 1, 3, 3100)]
    # Its first check's neighbours: on stream 0 the seed and two viewers
    # (40 and 20 ms), on stream 1 the seed and a viewer forecast before the
    # join alone; viewer 2's first check at its join's ms.
    FORECAST_C = [(3100, 3, 0, 0, 90, None, 1), (3100, 3, 1, 0, 40, None, 1), (3150, 3, 2, 0, 20, None, 1),
                  (3100, 3, 0, 1, 70, None, 1), (2500, 3, 1, 1, 5, None, 1)]
    CHECKS_C = [(3_200_000, 3, 0, 0, 90_000), (3_200_000, 3, 1, 0, 70_000), (2_000_000, 2, 0, 0, 50_000),
                (2_100_000, 2, 0, 0, 40_000)]
    # Runs u1, u2 (a testbed side) and n1 (a model's), cell d: viewers 2
    # and 3 link to the seed, 100 ms after the join and 300, 400 and 300.
    U_OPENS = {"u1": (100, 300), "u2": (100, 400), "n1": (100, 300)}

    @classmethod
    def setUpClass(cls):
        import duckdb

        from simlab import catalog

        consts = catalog.scoring_consts()
        text = catalog.CONST.sub(lambda m: str(consts[m.group(1)]), catalog.SCORING.read_text())
        conn = duckdb.connect()
        for m in re.finditer(r"^CREATE OR REPLACE MACRO sketch_\w+.*?;$", text, re.M | re.S):
            conn.execute(m.group(0))
        conn.execute("CREATE TABLE jobs (k VARCHAR, src VARCHAR, stored_cell VARCHAR, env MAP(VARCHAR, VARCHAR))")
        conn.execute("CREATE TABLE events (k VARCHAR, src VARCHAR, t_ms BIGINT, kind VARCHAR, node BIGINT)")
        conn.execute("CREATE TABLE haves (k VARCHAR, src VARCHAR, t_ms BIGINT, node BIGINT, peer BIGINT, "
                     "channel_open_ms BIGINT)")
        conn.execute("CREATE TABLE forecast (k VARCHAR, src VARCHAR, t_ms BIGINT, node BIGINT, peer BIGINT, "
                     "stream BIGINT, sample_ms BIGINT, mean_ms BIGINT, samples BIGINT)")
        conn.execute("CREATE TABLE parent_checks (k VARCHAR, src VARCHAR, t_us BIGINT, node BIGINT, "
                     "stream BIGINT, best_peer BIGINT, best_lag_us BIGINT)")

        def run(k, cell, events, haves=(), forecast=(), checks=()):
            conn.execute("INSERT INTO jobs VALUES (?, 's', ?, MAP {})", [k, cell])
            for table, rows in (("events", events), ("haves", haves), ("forecast", forecast),
                                ("parent_checks", checks)):
                if rows:
                    marks = ", ".join("?" * len(rows[0]))
                    conn.executemany(f"INSERT INTO {table} VALUES ('{k}', 's', {marks})", rows)

        run("a", "c", cls.EVENTS, cls.HAVES, cls.FORECAST, cls.CHECKS)
        run("b", "c", [])
        # Two runs of a testbed side (t1, t2) and one of a model's (m1), each
        # the fixture's run, its channels opening that much later; and a
        # model's run of no links between viewers (m2).
        for k, shift in (("t1", 0), ("t2", 20), ("m1", 50)):
            run(k, "c", cls.EVENTS, [(t + shift, n, p, o + shift) for t, n, p, o in cls.HAVES])
        run("m2", "c", cls.EVENTS, [h for h in cls.HAVES if h[1] == 0 or h[2] == 0])
        run("c", "c", cls.EVENTS_C, cls.HAVES_C, cls.FORECAST_C, cls.CHECKS_C)
        for k, (two, three) in cls.U_OPENS.items():
            run(k, "d", cls.EVENTS_C, [(2000 + two, 2, 0, 2000 + two), (3000 + three, 3, 0, 3000 + three)])
        names = ["twin_window_of", "side_window_of", "link_opens_of", "first_checks_of", "link_open_fit_of"]
        for macro in catalog.scoring_statements(
                names, {"events", "haves", "parent_checks", "forecast", "jobs.env", "jobs.stored_cell"}):
            conn.execute(macro)
        cls.conn = conn

    @staticmethod
    def side(*ks, cell="c"):
        return "[" + ", ".join(f"{{'cell': '{cell}', 'seed': 1, 'k': '{k}', 'src': 's'}}" for k in ks) + "]"

    def fit(self, a, b, cols="nb, measure, a_runs, b_runs, a_median, b_median, diff, spread, bound, pass", **kw):
        args = "".join(f", {name} := {value}" for name, value in kw.items())
        return self.conn.execute(f"SELECT {cols} FROM link_open_fit_of({a}, {b}{args})").fetchall()

    def test_the_bench_rule_bounds_the_model_by_the_testbeds_repeat_spread(self):
        rows = self.fit(self.side("m1"), self.side("t1", "t2"))
        # Viewer 2's links: the testbed's opens 300 and 320 ms after its
        # join (the seed's), 200 and 220 (viewer 1's), a median of two the
        # lower, its spread 20; the model's 50 ms past t1's, past 20 + 10.
        self.assertEqual(rows, [
            ("seed", "open_ms", 1, 2, 350.0, 300.0, -50.0, 20.0, 30.0, False),
            ("seed", "peer_open_ms", 1, 2, 360.0, 310.0, -50.0, 20.0, 30.0, False),
            ("viewer", "open_ms", 1, 2, 250.0, 200.0, -50.0, 20.0, 30.0, False),
            ("viewer", "peer_open_ms", 1, 2, 240.0, 190.0, -50.0, 20.0, 30.0, False),
        ])
        # A floor that covers the rest passes.
        rows = self.fit(self.side("m1"), self.side("t1", "t2"), cols="pass", floor_ms=30)
        self.assertEqual(rows, [(True,)] * 4)

    def test_a_class_the_model_lacks_fails(self):
        rows = self.fit(self.side("m2"), self.side("t1", "t2"),
                        cols="nb, measure, a_runs, b_runs, a_links, b_links, a_median, b_median, diff, pass")
        self.assertEqual(rows, [
            ("seed", "open_ms", 1, 2, 1, 2, 300.0, 300.0, 0.0, True),
            ("seed", "peer_open_ms", 1, 2, 1, 2, 310.0, 310.0, 0.0, True),
            ("viewer", "open_ms", None, 2, None, 2, None, 200.0, None, False),
            ("viewer", "peer_open_ms", None, 2, None, 2, None, 190.0, None, False),
        ])

    def test_the_spread_is_of_each_runs_median(self):
        # u1's links 100 and 300, u2's 100 and 400: both runs' medians (of
        # two, the lower) 100, a spread of none, where their means' would be
        # 50; the cell is each run's job's, whatever the side names.
        rows = self.fit(self.side("n1", cell="x"), self.side("u1", "u2", cell="x"),
                        cols="cell, nb, measure, a_links, b_links, a_median, b_median, spread, bound, pass")
        self.assertEqual(rows, [("d", "seed", "open_ms", 2, 4, 100.0, 100.0, 0.0, 10.0, True)])

    def test_a_side_of_no_runs_fails(self):
        import duckdb

        none = "(SELECT list({'k': k}) FROM jobs WHERE src = 'typo')"
        for sql in (f"FROM link_open_fit_of([], {self.side('t1')})",
                    f"FROM link_open_fit_of({none}, {self.side('t1')})",
                    f"FROM link_open_fit_of({self.side('m1')}, {none})",
                    "FROM link_opens_of([])", f"FROM link_opens_of({none})", "FROM first_checks_of([])"):
            with self.subTest(sql=sql), self.assertRaisesRegex(duckdb.Error, r"_of: a side of no runs"):
                self.conn.execute(sql).fetchall()

    def test_each_joiners_links_in_ms_after_its_join(self):
        cols = "node, peer, nb, arrival, open_ms, first_have_ms, peer_open_ms, open_rank, links"
        rows = self.conn.execute(f"SELECT {cols} FROM link_opens_of([{{'k': 'a'}}]) ORDER BY node, peer").fetchall()
        self.assertEqual(rows, [
            (1, 0, "seed", 1, 150, 160, 140, 1, 1),
            (2, 0, "seed", 2, 300, 350, 310, 2, 2),
            (2, 1, "viewer", 2, 200, 210, 190, 1, 2),
        ])
        self.assertEqual(self.conn.execute("SELECT count(*) FROM link_opens_of([{'k': 'b'}])").fetchone(), (0,))
        # Run c: of viewer 3's channels, the one that opened within 50 ms
        # before its join alone; and of viewer 1's with it, the one in the
        # span. Viewer 1's with viewer 3, a later joiner, isn't its link.
        rows = self.conn.execute(f"SELECT {cols} FROM link_opens_of([{{'k': 'c'}}]) ORDER BY node, peer").fetchall()
        self.assertEqual(rows, [(3, 1, "viewer", 3, -30, 5, 100, 1, 1)])

    def test_the_first_check_reads_each_neighbours_last_forecast(self):
        cols = ("node, arrival, check_ms, streams, best_seed, seed_ms, best_viewer_ms, seed_minus_best_ms, "
                "best_lag_ms")
        rows = self.conn.execute(f"SELECT {cols} FROM first_checks_of([{{'k': 'a'}}])").fetchall()
        # Stream 0: the seed's 100 moved an eighth toward 120, the viewer's
        # 10; stream 1: the seed's 80 and no viewer's. Medians of two take
        # the lower.
        self.assertEqual(rows, [(2, 2, 400.0, 2, 0.5, 80.0, 10.0, 92.5, 10.0)])
        # Run c: viewer 2's first check the one at its join's ms, with no
        # forecasts; viewer 3's stream 0's best viewer the nearer of two
        # (20), stream 1's none (its one viewer's forecast came before the
        # join); both streams' best the seed.
        rows = self.conn.execute(f"SELECT {cols} FROM first_checks_of([{{'k': 'c'}}]) ORDER BY node").fetchall()
        self.assertEqual(rows, [(2, 2, 0.0, 1, 1.0, None, None, None, 50.0),
                                (3, 3, 200.0, 2, 1.0, 70.0, 20.0, 70.0, 70.0)])


if __name__ == "__main__":
    unittest.main()
