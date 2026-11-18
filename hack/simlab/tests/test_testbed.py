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
           arrival order (soonest first)
  Tier     the tier's cells are hostable, honest, of the robust and
           browsers tiers; hack/sim-farm/matrices/testbed-tier is what
           tier_matrices writes
  Replays  a run's two shaper replays (simlab.bench), of its every offer
           and of what its link carried, are jobs of its source's cell,
           numbered as an ingest numbers its runs, keyed by their code,
           their source and their offers, and never written over a source
  ShaperFit  scoring.sql's shaper_fit_of over a fixture: the drops' rules
           read the offered replay, the departures' the carried one, a
           link with slots its delays' distance; a source with two
           replays of a kind fails

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
        # go back, key the run apart.
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
                                                  + "QDISC=clsact>ifb-netem>root-class\n" + stamps)
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
            self.assertTrue((d / "tb-a" / "replay" / "packets.csv").exists())
            self.assertTrue((d / "tb-a" / "replay-carried" / "packets.csv").exists())
            self.assertTrue((d / "cache" / "testbed" / "r").exists())
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
        for k, src, env in (("tb", "testbed/s", {}),
                            ("ro", "testbed/r", {"SOURCE_K": "tb", "REPLAY_OFFERS": "offered"}),
                            ("rc", "testbed/r", {"SOURCE_K": "tb", "REPLAY_OFFERS": "carried"}),
                            ("ro2", "testbed/r2", {"SOURCE_K": "tb", "REPLAY_OFFERS": "offered"})):
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

if __name__ == "__main__":
    unittest.main()
