#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.neutral: what moved between two runs' job dirs (identical; a
column's values, expected or not; a `.zst` table; a table gone or new;
rows inserted; a new column; the row's fields and the trace; series_meta)
in flat memory, the check end to end over stand-in sims (exit statuses,
kept runs, a failed run), and the commits it checks (a merge's parents,
the working tree's snapshot) in a scratch repository:

  python3 hack/simlab/tests/test_neutral.py
"""

import json
import subprocess
import sys
import tempfile
import tracemalloc
import unittest
from compression import zstd
from pathlib import Path
from unittest import mock

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import neutral  # noqa: E402
from simlab import suite as lab  # noqa: E402

ASKS = ["t_ms", "node", "ask", "rtt_ms", "outcome"]
ROW = {"trace": "aaaa", "reads_hash": "bbbb", "timeouts": 3, "up_wire": 1000, "_wall_s": 1.5, "_rss_mb": 100,
       "wall_secs": 1.2}


def asks(n=50):
    return [[100 + i, i % 4, i, 20 + i % 7, "answered" if i % 5 else "expired"] for i in range(n)]


def tables():
    return {"asks": (ASKS, asks()),
            "seconds": (["t", "lag_p99_ms"], [[t, 100 + t] for t in range(10)]),
            "parents": (["t_ms", "node", "parent"], [[t * 10, t, 0] for t in range(5)])}


def csv_text(head, rows):
    return ",".join(head) + "\n" + "".join(",".join(map(str, r)) + "\n" for r in rows)


def job(root, name, tabs=None, row=None, meta=None, zst=()):
    """A kept run's dir as neutral.run keeps one: tables/ (each table's CSV,
    those of `zst` compressed, and its schema) and its record."""
    t = root / name / "tables"
    t.mkdir(parents=True)
    for table, (head, rows) in (tables() if tabs is None else tabs).items():
        text = csv_text(head, rows).encode()
        if table in zst:
            (t / f"{table}.csv.zst").write_bytes(zstd.compress(text))
        else:
            (t / f"{table}.csv").write_bytes(text)
        (t / f"{table}.schema.json").write_text(json.dumps({"columns": [{"name": c, "type": "BIGINT"} for c in head]}))
    meta = {"sim": "strims-sim", "asks_lost": 0} if meta is None else meta
    (t / "series_meta.json").write_text(json.dumps(meta))
    rec = {"sha": name, "cell": "c", "seed": 1, "row": {**ROW, **(row or {})}, "files": neutral.manifest(t)}
    return neutral.Run(root / name, rec)


class Compare(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="neutral-")
        self.root = Path(self.tmp.name)
        self.work = mock.patch.object(neutral, "WORK", self.root / "work")
        self.work.start()

    def tearDown(self):
        self.work.stop()
        self.tmp.cleanup()

    def pair(self, base=None, head=None, **kw):
        a = job(self.root, "a", **(base or {}))
        b = job(self.root, "b", **(head or {}), **kw)
        return a, b

    def test_identical(self):
        a, b = self.pair(head={"row": {"_wall_s": 9.0, "_rss_mb": 900, "wall_secs": 8.0}})
        self.assertEqual(neutral.compare(a, b, neutral.Expect()), [])

    def test_column_change(self):
        t = tables()
        rows = t["asks"][1]
        for i in (3, 7):
            rows[i][3] += 1
        a, b = self.pair(head={"tabs": t})
        moves = neutral.compare(a, b, neutral.Expect())
        self.assertEqual([(m.name, m.kind, m.expected) for m in moves], [("asks.rtt_ms", "value", False)])
        m = moves[0]
        self.assertEqual((m.detail["rows"], m.detail["of"]), (2, 50))
        self.assertEqual([(s["row"], s["base"], s["head"]) for s in m.detail["sample"]],
                         [(3, "23", "24"), (7, "20", "21")])
        self.assertEqual(m.detail["sample"][0]["at"], "t_ms=103 node=3 ask=3")
        for names in (["asks.rtt_ms"], ["asks"], ["seconds,asks.rtt_ms"]):
            e = neutral.Expect(names)
            self.assertEqual([x.expected for x in neutral.compare(a, b, e)], [True], names)
            self.assertEqual(e.unused(), ["seconds"] if names == ["seconds,asks.rtt_ms"] else [])
        self.assertEqual([x.expected for x in neutral.compare(a, b, neutral.Expect(["asks.ask"]))], [False])

    def test_unexpected_beside_expected(self):
        t = tables()
        t["asks"][1][4][3] = 99
        t["seconds"][1][2][1] = 0
        a, b = self.pair(head={"tabs": t})
        moves = neutral.compare(a, b, neutral.Expect(["asks.rtt_ms"]))
        self.assertEqual([(m.name, m.expected) for m in moves], [("asks.rtt_ms", True), ("seconds.lag_p99_ms", False)])
        self.assertIn("seconds.lag_p99_ms: 1 of 10 rows differ, first row 2: 102 -> 0 (t=2 lag_p99_ms=102)",
                      moves[1].text)

    def test_zst(self):
        a, b = self.pair(head={"zst": ("asks",)})
        self.assertEqual(b.files["asks.csv"]["file"], "asks.csv.zst")
        self.assertEqual(neutral.compare(a, b, neutral.Expect()), [])
        t = tables()
        t["asks"][1][30][4] = "elsewhere"
        b2 = job(self.root, "b2", tabs=t, zst=("asks",))
        a2 = job(self.root, "a2", zst=("asks",))
        moves = neutral.compare(a2, b2, neutral.Expect())
        self.assertEqual([(m.name, m.detail["rows"]) for m in moves], [("asks.outcome", 1)])
        self.assertEqual(moves[0].detail["sample"][0]["head"], "elsewhere")

    def test_missing_and_new_table(self):
        t = tables()
        del t["parents"]
        t["haves"] = (["t_ms", "node"], [[1, 2]])
        a, b = self.pair(head={"tabs": t})
        moves = neutral.compare(a, b, neutral.Expect())
        self.assertEqual([(m.name, m.kind, m.expected) for m in moves],
                         [("haves", "new", False), ("parents", "gone", False)])
        self.assertEqual(moves[1].text, "parents: gone (table, 5 rows)")
        e = neutral.Expect(["haves", "parents"])
        self.assertTrue(all(m.expected for m in neutral.compare(a, b, e)))
        self.assertEqual(e.unused(), [])

    def test_rows_inserted(self):
        t = tables()
        rows = t["asks"][1]
        rows[40][4] = "closed"
        rows[20:20] = [[555, 9, 900 + i, 1, "answered"] for i in range(3)]
        a, b = self.pair(head={"tabs": t})
        moves = neutral.compare(a, b, neutral.Expect())
        self.assertEqual([(m.name, m.kind) for m in moves], [("asks", "rows")])
        d = moves[0].detail
        self.assertEqual((d["rows"], d["only_base"], d["only_head"]), ([50, 53], 1, 4))
        self.assertEqual(d["first"]["row"], 20)
        self.assertEqual(d["first"]["head"]["t_ms"], "555")
        self.assertEqual([r["row"] for r in d["sample_head"]], [20, 21, 22, 43])
        self.assertEqual(d["sample_base"][0]["row"], 40)
        self.assertEqual(d["pairs"], [{"base_row": 40, "head_row": 43, "columns": ["outcome"]}])
        self.assertIn("asks: rows 50 -> 53, 4 only in head and 1 only in base, first divergence at row 20; "
                      "near pairs differ in outcome", moves[0].text)
        self.assertTrue(neutral.compare(a, b, neutral.Expect(["asks"]))[0].expected)
        # A column expected to move doesn't make its rows move.
        t2 = tables()
        t2["asks"][1].append([999, 1, 50, 1, "answered"])
        for r in t2["asks"][1][:10]:
            r[3] = 0
        b2 = job(self.root, "b2", tabs=t2)
        d2 = neutral.compare(a, b2, neutral.Expect(["asks.rtt_ms"]))[0].detail
        self.assertEqual((d2["only_base"], d2["only_head"]), (0, 1))

    def test_renumbered(self):
        """Rows inserted before a column that numbers the rows move every
        later row; leaving the column out shows the insertion."""
        t = tables()
        rows = t["asks"][1]
        rows[20:20] = [[555, 9, 0, 1, "answered"] for _ in range(3)]
        for i, r in enumerate(rows):
            r[2] = i
        a, b = self.pair(head={"tabs": t})
        [m] = neutral.compare(a, b, neutral.Expect())
        d = m.detail
        self.assertEqual((d["only_base"], d["only_head"]), (30, 33))
        self.assertEqual(d["without"], {"column": "ask", "only_base": 0, "only_head": 3})
        self.assertEqual([r["row"] for r in d["sample_head"]], [20, 21, 22])
        self.assertEqual(d["sample_base"], [])
        self.assertIn("; without ask, 3 only in head and 0 only in base", m.text)
        self.assertEqual(neutral.divergence_text(d["first"]),
                         ["first divergence, row 20 in base: t_ms=120 node=0 ask=20 rtt_ms=26 outcome=expired",
                          "first divergence, row 20 in head: t_ms=555 node=9 ask=20 rtt_ms=1 outcome=answered"])
        # The columns of a first divergence where few differ are named,
        # base -> head.
        first = {"row": 4, "base": {"t": "1", "x": "2", "y": "", "z": "5"},
                 "head": {"t": "1", "x": "3", "y": "4", "z": "5"}}
        self.assertEqual(neutral.divergence_text(first),
                         ["first divergence, row 4 (t=1 x=2 y=''): x 2 -> 3, y '' -> 4"])

    def test_new_column(self):
        t = tables()
        t["asks"] = (ASKS + ["since_ms"], [r + [7] for r in asks()])
        a, b = self.pair(head={"tabs": t})
        moves = neutral.compare(a, b, neutral.Expect())
        self.assertEqual([(m.name, m.kind, m.expected) for m in moves], [("asks.since_ms", "new", False)])
        self.assertTrue(neutral.compare(a, b, neutral.Expect(["asks.since_ms"]))[0].expected)

    def test_row_fields_and_trace(self):
        a, b = self.pair(head={"row": {"timeouts": 4, "budget": 1}})
        moves = neutral.compare(a, b, neutral.Expect())
        self.assertEqual([(m.name, m.kind, m.text) for m in moves],
                         [("row.budget", "new", "row.budget: new (1)"),
                          ("row.timeouts", "value", "row.timeouts: 3 -> 4")])
        self.assertTrue(all(m.expected for m in neutral.compare(a, b, neutral.Expect(["row"]))))
        t = tables()
        t["asks"][1][0][2] = 77
        b2 = job(self.root, "b2", tabs=t, row={"trace": "cccc"})
        moves = neutral.compare(a, b2, neutral.Expect(["row"]))
        self.assertEqual([(m.name, m.kind, m.expected) for m in moves],
                         [("row.trace", "value", False), ("asks", "bytes", False)])
        self.assertIn("not diagnosed: the trace moved", moves[1].text)
        self.assertTrue(neutral.compare(a, b2, neutral.Expect(["row.trace", "asks"]))[0].expected)

    def test_series_meta(self):
        a, b = self.pair(head={"meta": {"sim": "strims-sim", "asks_lost": 2, "haves_lost": 0}})
        moves = neutral.compare(a, b, neutral.Expect(["series_meta.haves_lost"]))
        self.assertEqual([(m.name, m.kind, m.expected) for m in moves],
                         [("series_meta.asks_lost", "value", False), ("series_meta.haves_lost", "new", True)])

    def test_flat_memory(self):
        """A table diagnosed whole would hold every row; streaming holds a
        partition's hashes at a time, so four times the rows take no more
        memory."""
        head = ["t_ms", "node", "peer", "seq", "lag_ms", "how"]
        peaks = []
        for n in (100_000, 400_000):
            rows = [[i, i % 31, (i * 7) % 31, i // 31, i % 997, "push"] for i in range(n)]
            moved = [r[:] for r in rows]
            for i in range(0, n, 2000):
                moved.insert(i, [i, 99, 99, 0, 0, "answer"])
                moved[i + 1000][4] = -1
            a = job(self.root, f"a{n}", {"asks": (head, rows)}, zst=("asks",))
            b = job(self.root, f"b{n}", {"asks": (head, moved)}, zst=("asks",))
            del rows, moved
            with mock.patch.object(neutral, "PART_ROWS", 10_000):
                tracemalloc.start()
                moves = neutral.compare(a, b, neutral.Expect())
                peaks.append((tracemalloc.get_traced_memory()[1], a.files["asks.csv"]["bytes"]))
                tracemalloc.stop()
            self.assertEqual((moves[0].detail["only_base"], moves[0].detail["only_head"]), (n // 2000, n // 1000))
            self.assertEqual(list(moves[0].detail["pair_columns"]), ["lag_ms"])
        (small, _), (large, size) = peaks
        self.assertLess(large, 1.25 * small + (256 << 10), peaks)
        self.assertLess(large, size // 2, peaks)


# A sim as suite.run_jobs runs one: its variant (this dir's VARIANT file)
# writes the job's tables into its output dir and prints its row; it counts
# its runs in RUNS.
FAKE = f"""#!{sys.executable}
import json, os, sys
here = os.path.dirname(os.path.abspath(__file__))
variant = open(os.path.join(here, "VARIANT")).read().strip()
seed = int(os.environ["STRIMS_SIM_SEED"])
with open(os.path.join(here, "RUNS"), "a") as f:
    f.write(f"{{seed}}\\n")
if variant == "fail":
    sys.exit("no row")
out = os.environ["STRIMS_SIM_OUT"]
assert os.environ["STRIMS_SIM_SPEC"] == "1" and os.environ["STRIMS_SIM_BIN_NODES"] == "1"
rtt = 21 if variant == "moved" and seed == 2 else 20
with open(os.path.join(out, "asks.csv"), "w") as f:
    f.write("t_ms,node,rtt_ms\\n" + "".join(f"{{t}},1,{{rtt if t == 5 else 20}}\\n" for t in range(10)))
open(os.path.join(out, "series_meta.json"), "w").write(json.dumps({{"seed": seed}}))
print("ROW " + json.dumps({{"trace": f"t{{seed}}", "reads_hash": "r", "timeouts": seed, "wall_secs": 0.1}}))
"""


class Check(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="neutral-check-")
        self.root = Path(self.tmp.name)
        self.bins = {}
        for sha, variant in (("a" * 40, "same"), ("b" * 40, "moved")):
            d = self.root / "bin" / sha[:12]
            d.mkdir(parents=True)
            (d / lab.SIM).write_text(FAKE)
            (d / lab.SIM).chmod(0o755)
            (d / "VARIANT").write_text(variant)
            self.bins[sha] = d
        self.cell = lab.cell("fake", lab.SIM, "calibration_star", {"STRIMS_SIM_JOB": "1"}, 1, "a stand-in")
        self.patches = [mock.patch.object(neutral, "WORK", self.root / "work"),
                        mock.patch.object(neutral, "build", lambda sha, log=None: (self.bins[sha], 1.0)),
                        mock.patch.object(neutral, "pairs", lambda bases, head: ("b" * 40, ["a" * 40]))]
        for p in self.patches:
            p.start()

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.tmp.cleanup()

    def check(self, expect=()):
        return neutral.check(["a"], "b", neutral_cells([self.cell]), [1, 2], expect, slots=2, mem_gib=4,
                             log=lambda s: None)

    def runs(self, sha):
        f = self.bins[sha] / "RUNS"
        return f.read_text().split() if f.exists() else []

    def test_check(self):
        r = self.check()
        self.assertEqual(r.exit_code(), 1)
        [p] = r.pairs
        self.assertEqual([(d.seed, [m.name for m in d.moves]) for d in p.runs],
                         [(1, []), (2, ["asks.rtt_ms"])])
        text = r.text()
        self.assertIn("fake 1: the same (trace t1, reads r, 2 files)", text)
        self.assertIn("UNEXPECTED asks.rtt_ms: 1 of 10 rows differ, first row 5: 20 -> 21 (t_ms=5 node=1 rtt_ms=20)",
                      text)
        self.assertIn("every move: --expect asks.rtt_ms", text)
        self.assertIn("result: UNEXPECTED MOVES (exit 1)", text)
        self.assertEqual(sorted(self.runs("a" * 40)), ["1", "2"])
        # Kept: a second check runs nothing.
        r = self.check(["asks.rtt_ms"])
        self.assertEqual(r.exit_code(), 0)
        self.assertEqual(sorted(self.runs("b" * 40)), ["1", "2"])
        self.assertIn("expected asks.rtt_ms: 1 of 10 rows differ", r.text())
        j = r.json()
        self.assertEqual((j["exit"], j["unexpected"], j["suggested_expect"]), (0, 0, ["asks.rtt_ms"]))
        self.assertEqual(j["pairs"][0]["runs"][1]["moves"][0]["detail"]["sample"][0]["row"], 5)
        json.dumps(j)

    def test_failed_run(self):
        (self.bins["b" * 40] / "VARIANT").write_text("fail")
        r = self.check()
        self.assertEqual(r.exit_code(), 2)
        self.assertEqual(len(r.failures), 2)
        self.assertIn("not compared: no run at bbbbbbbbbbbb", r.text())
        # Nothing of a failed run is kept: it runs again.
        (self.bins["b" * 40] / "VARIANT").write_text("same")
        self.assertEqual(self.check().exit_code(), 0)
        self.assertEqual(sorted(self.runs("b" * 40)), ["1", "1", "2", "2"])


def neutral_cells(cells):
    return [{**c, "env": {**c["env"], **neutral.LOGS[c["sim"]]}} for c in cells]


class Commits(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix="neutral-git-")
        self.repo = Path(self.tmp.name)
        self.patches = [mock.patch.object(neutral, "ROOT", self.repo),
                        mock.patch.object(neutral, "WORK", self.repo / "target" / "neutral")]
        for p in self.patches:
            p.start()
        g = self.git
        g("init", "-q", "-b", "main")
        g("config", "user.name", "T")
        g("config", "user.email", "t@example.com")
        (self.repo / ".gitignore").write_text("target/\n")
        (self.repo / "a.txt").write_text("a\n")
        g("add", "-A")
        g("commit", "-qm", "A")
        g("checkout", "-qb", "side")
        (self.repo / "b.txt").write_text("b\n")
        g("add", "-A")
        g("commit", "-qm", "B")
        g("checkout", "-q", "main")
        (self.repo / "c.txt").write_text("c\n")
        g("add", "-A")
        g("commit", "-qm", "C")
        g("merge", "-q", "--no-edit", "side")

    def tearDown(self):
        for p in self.patches:
            p.stop()
        self.tmp.cleanup()

    def git(self, *args):
        return subprocess.run(["git", *args], cwd=self.repo, check=True, capture_output=True, text=True).stdout.strip()

    def test_pairs(self):
        sha = lambda r: self.git("rev-parse", r)
        a, b, c, m = sha("main~1~1"), sha("side"), sha("main~1"), sha("main")
        self.assertEqual(neutral.pairs([], "main"), (m, [c, b]))
        self.assertEqual(neutral.pairs([c], "main"), (m, [c, b]))
        self.assertEqual(neutral.pairs([a], "main"), (m, [a, c, b]))
        self.assertEqual(neutral.pairs([], "main~1"), (c, [a]))
        self.assertEqual(neutral.pairs(["side"], "main~1"), (c, [b]))
        with self.assertRaises(neutral.GitError):
            neutral.pairs([], "main~1~1")
        with self.assertRaises(neutral.GitError):
            neutral.pairs(["nope"], "main")

    def test_snapshot(self):
        head = self.git("rev-parse", "HEAD")
        self.assertEqual(neutral.snapshot(), head)
        (self.repo / "a.txt").write_text("a2\n")
        (self.repo / "new.txt").write_text("n\n")
        (self.repo / "target").mkdir(exist_ok=True)
        (self.repo / "target" / "x").write_text("ignored\n")
        status = self.git("status", "--porcelain")
        s = neutral.snapshot()
        self.assertNotEqual(s, head)
        self.assertEqual(self.git("rev-parse", f"{s}^"), head)
        self.assertEqual(self.git("show", f"{s}:a.txt"), "a2")
        self.assertEqual(self.git("show", f"{s}:new.txt"), "n")
        self.assertEqual(subprocess.run(["git", "cat-file", "-e", f"{s}:target/x"], cwd=self.repo,
                                        capture_output=True).returncode, 128)
        # The checkout's index and files are as they were; the same tree
        # gives the same commit.
        self.assertEqual(self.git("status", "--porcelain"), status)
        self.assertEqual(self.git("diff", "--cached", "--name-only"), "")
        self.assertEqual(neutral.snapshot(), s)
        self.assertEqual(neutral.pairs([], "."), (s, [head]))


class Cells(unittest.TestCase):
    def test_sets(self):
        audit = neutral.cell_set()
        self.assertEqual([c["name"] for c in audit],
                         ["star-31-5", "halve-15-10", "thin-15-10-trickle", "star-15-10-end-abrupt", "no-greedy",
                          "grabbers", "churn", "wan-mobile", "cable-end-abrupt"])
        for c in audit:
            for k, v in neutral.LOGS[c["sim"]].items():
                self.assertEqual(c["env"][k], v)
        self.assertEqual(neutral.LOGS[lab.SIM], {"STRIMS_SIM_FORECAST": "1", "STRIMS_SIM_SPEC": "1",
                                                 "STRIMS_SIM_BIN_NODES": "1"})
        # Cells by name, the audit's first, then the tiers'.
        two = neutral.cell_set("star-31-5,jitter-15-10")
        self.assertEqual(two[0]["env"]["STRIMS_SIM_CAL_SECS"], "30")
        self.assertEqual(two[1]["name"], "jitter-15-10")
        self.assertEqual(len(neutral.cell_set("smoke")), len(lab.TIERS["smoke"]))
        with self.assertRaises(ValueError):
            neutral.cell_set("star-31-5,nope")


if __name__ == "__main__":
    unittest.main()
