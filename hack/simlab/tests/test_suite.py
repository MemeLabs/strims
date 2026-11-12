#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.suite: write_matrices called as the console calls it writes what
`suite.py farm --no-run` writes (golden_suite's golden), and run_cells
runs a cell's seeds under a binary, folds their tables into the cache (in
the duckdb module) as each ends, with its binary's commit, dirty flag and
sha256, indexes them and gives their sides, keeping them for the next call,
over a scratch cache and main checkout with a stand-in sim that writes
golden_cli's job dirs:

  python3 hack/simlab/tests/test_suite.py
"""

import hashlib
import io
import json
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import golden_cli
import golden_suite
import goldens

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import store, store_host  # noqa: E402
from simlab import suite as lab  # noqa: E402

# A sim binary as run_jobs runs one: it writes the job dir made for its
# seed into its output dir and prints its row, or fails without one.
SIM = f"""#!{sys.executable}
import json, os, shutil, sys
d = os.path.join(os.environ["FAKE_JOBS"], os.environ["STRIMS_SIM_SEED"])
if not os.path.isdir(d):
    sys.exit("no job for this seed")
shutil.copytree(d, os.environ["STRIMS_SIM_OUT"], dirs_exist_ok=True, ignore=shutil.ignore_patterns("row.json"))
print("ROW " + open(os.path.join(d, "row.json")).read())
"""


class Matrices(unittest.TestCase):
    def written(self, **kw):
        """The files write_matrices writes for `kw` over the smoke tier,
        as golden_suite names them, and its Matrices."""
        with tempfile.TemporaryDirectory(prefix="simlab-matrices-") as tmp:
            tmp = Path(tmp)
            with mock.patch.object(lab, "WORK", tmp / "work"), \
                    mock.patch.object(lab, "farm_root", lambda: tmp / "main"):
                m =lab.write_matrices(lab.cells("smoke"), range(2001, 2033), **kw)
            text = "".join(f"=== {f.relative_to(tmp)}\n{f.read_text()}"
                           for f in sorted(p for p in tmp.rglob("*") if p.is_file()))
            return text, m, tmp

    def golden(self, name):
        return goldens.read(golden_suite.GOLDEN)[name].split("--- exit 0\n", 1)[1]

    def test_sets_as_the_cli_writes_them(self):
        text, m, tmp = self.written(sets=["x", "y,z"], ref=golden_suite.A,
                                    knob={"strims-sim": "STRIMS_SIM_EXP", "ppspp": "SIM_EXP"})
        self.assertEqual(text, self.golden("smoke-sets"))
        cells = lab.cells("smoke")
        self.assertEqual([(Path(x.path).relative_to(tmp / "work" / "farm").name, x.cell, x.env) for x in m.matrices],
                         [(f"{m.name}-{i:02d}.toml", c["name"], c["env"]) for i, c in enumerate(cells)])
        self.assertEqual((m.index["base"], m.index["values"], m.index["knob"]),
                         ("off", ["off", "x", "y,z"], {"ppspp": "SIM_EXP", "strims-sim": "STRIMS_SIM_EXP"}))

    def test_refs_as_the_cli_writes_them(self):
        text, m, _ = self.written(arms={"base": golden_suite.A, "new": golden_suite.B})
        self.assertEqual(text, self.golden("smoke-refs"))
        self.assertEqual((m.index["arms"], m.index["base"]), ({"base": golden_suite.A, "new": golden_suite.B}, "base"))
        self.assertNotIn("knob", m.index)

    def test_cells(self):
        every = lab.cells("peers", ["churn", "star-15"], ["SIM_DEPART=60:40:10", "X=a=b"])
        self.assertEqual([c["name"] for c in every], ["churn", "star-15"])
        self.assertEqual([(c["env"]["SIM_DEPART"], c["env"]["X"]) for c in every], [("60:40:10", "a=b")] * 2)
        self.assertEqual(lab.cells("smoke"), lab.TIERS["smoke"])


class LocalRuns(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-runs-")
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        jobs = root / "jobs"
        for seed in (1, 2):
            job = golden_cli.JOBS[2][:6] + (seed,) + golden_cli.JOBS[2][7:]
            j = golden_cli.job_dir(jobs / str(seed), job)
            (jobs / str(seed) / "row.json").write_text(json.dumps(j["row"]))
        # The cache and the catalog's main checkout are the scratch dir's
        # (golden_cli's fixtures load simdb_test, which sets simlab.suite's
        # simdb: its own).
        sm = lab.simdb()
        (root / "main").mkdir()
        for p in (mock.patch.object(lab, "_simdb", sm), mock.patch.object(lab, "_catalog", None),
                  mock.patch.object(lab, "WORK", root / "work"),
                  mock.patch.object(sm, "main_checkout", lambda: root / "main"),
                  mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(root / "sim-db")})):
            p.start()
            self.addCleanup(p.stop)
        self.bin = root / "bin"
        self.bin.mkdir()
        (self.bin / lab.SIM).write_text(SIM)
        (self.bin / lab.SIM).chmod(0o755)
        self.cell = lab.cell("halve-5", lab.SIM, "calibration_star", {**golden_cli.SS_ENV, "FAKE_JOBS": str(jobs)}, 1,
                             "a halving")

    def run_cells(self):
        runs = {"fake": (self.bin, lambda c: [1, 2, 3], lab.SIM, {})}
        real, self.ran = store.execute, []

        def execute(*a, **kw):
            self.ran.append(a)
            return real(*a, **kw)

        with mock.patch.object(sys, "stderr", io.StringIO()), mock.patch.object(store, "execute", execute):
            got = lab.run_cells([self.cell], runs, 2, 4)
        # The CLI is back for what follows.
        self.assertIsNone(store_host.EXECUTOR)
        return got

    def test_runs_fold_and_keep(self):
        rows, failures = self.run_cells()
        # The invocation's store folded and exported in the module.
        self.assertTrue(self.ran)
        self.assertEqual(len(failures), 1)
        self.assertRegex(failures[0], r"^fake halve-5 seed 3: exit 1: .*no job for this seed")
        side = rows["fake"]
        self.assertEqual(set(side), {("halve-5", 1), ("halve-5", 2)})
        for (_, seed), (k, src) in side.items():
            self.assertEqual(k, lab.job_id("fake", self.cell, seed))
            self.assertRegex(src, rf"^suite/{lab.ROOT.name}/\d{{8}}T\d{{6}}-\d+-\d+$")
        self.assertEqual(lab.held([k for k, _ in side.values()]), {k for k, _ in side.values()})
        kept = lab.load_rows(lab.WORK / "rows" / "fake.jsonl")
        self.assertEqual({r["id"]: (r["seed"], r["trace"]) for r in kept.values()},
                         {k: (seed, f"tr-{golden_cli.JOBS[2][1]}") for (_, seed), (k, _) in side.items()})
        # The next call runs only the seed it lacks, and keeps the rest.
        again, failures = self.run_cells()
        self.assertEqual(again, rows)
        self.assertEqual(len(failures), 1)

    def test_jobs_carry_their_binary(self):
        """A recorded binary's jobs come out of the fold and the index with
        the commit it was built from, its dirty flag and its sha256: in
        the store's runs, in jobs, as their experiment's arm, and in
        commits; a binary without a record has none but its sha256."""
        head = lab.git("rev-parse", "HEAD")
        digest = hashlib.sha256((self.bin / lab.SIM).read_bytes()).hexdigest()
        lab.record_build(self.bin, head, True)
        # A record already there stays; the binary's own sha256 is hashed.
        lab.record_build(self.bin, "0" * 40, False, replace=False)
        self.assertEqual(lab.provenance(self.bin, lab.SIM), {"commit": head, "dirty": True, "binary_sha256": digest})
        bare = self.bin.parent / "bare"
        bare.mkdir()
        (bare / lab.SIM).write_text(SIM)
        (bare / lab.SIM).chmod(0o755)
        runs = {"fake": (self.bin, lambda c: [1, 2], lab.SIM, {}), "bare": (bare, lambda c: [1], lab.SIM, {})}
        with mock.patch.object(sys, "stderr", io.StringIO()):
            rows, failures = lab.run_cells([self.cell], runs, 2, 4)
        self.assertEqual(failures, [])
        (src,) = {s for side in rows.values() for _, s in side.values()}
        stored = store.execute(f"SELECT k, commit, dirty, binary_sha256 FROM read_parquet("
                               f"{store_host.q(lab.Invocation.dest(src.rsplit('/', 1)[1]) / 'runs.parquet')}) "
                               "ORDER BY k;")
        bare_digest = hashlib.sha256((bare / lab.SIM).read_bytes()).hexdigest()
        want = {lab.job_id("fake", self.cell, s): (head, True, digest) for s in (1, 2)}
        want[lab.job_id("bare", self.cell, 1)] = (None, None, bare_digest)
        self.assertEqual({r["k"]: (r["commit"], r["dirty"], r["binary_sha256"]) for r in stored}, want)
        cat = lab.catalog()
        got = cat.lake("SELECT k, commit, dirty, binary_sha256, origin FROM jobs ORDER BY k;")
        self.assertEqual({r["k"]: (r["commit"], r["dirty"], r["binary_sha256"]) for r in got}, want)
        self.assertEqual({r["origin"] for r in got}, {"suite"})
        (exp,) = cat.lake(f"SELECT kind, arms, arm_labels FROM experiments WHERE exp_id = {store_host.q(src)};")
        self.assertEqual((exp["kind"], exp["arms"], exp["arm_labels"]), ("local", {"fake": head}, ["bare", "fake"]))
        (c,) = cat.lake("SELECT c.sha, c.subject FROM jobs j JOIN commits c ON c.sha = j.commit GROUP BY ALL;")
        self.assertEqual((c["sha"], c["subject"]), (head, lab.git("log", "-1", "--format=%s", head)))

    def test_each_job_folds_as_it_ends(self):
        """One slot, seed 3 (which fails) first: each job folds once it
        ends, alone, and the output dirs of the jobs before it, folded or
        failed, are gone by then."""
        sm = lab.simdb()
        real, folds = sm.fold, []

        def fold(db, jobs, src, **kw):
            jobs = list(jobs)
            out = Path(jobs[0]["dir"]).parent
            folds.append(([j["seed"] for j in jobs], len(list(out.iterdir()))))
            return real(db, jobs, src, **kw)

        runs = {"fake": (self.bin, lambda c: [3, 1, 2], lab.SIM, {})}
        with mock.patch.object(sys, "stderr", io.StringIO()), mock.patch.object(sm, "fold", fold):
            rows, failures = lab.run_cells([self.cell], runs, 1, 4)
        self.assertEqual(folds, [([1], 2), ([2], 1)])
        self.assertEqual(set(rows["fake"]), {("halve-5", 1), ("halve-5", 2)})
        self.assertEqual(len(failures), 1)


if __name__ == "__main__":
    unittest.main()
