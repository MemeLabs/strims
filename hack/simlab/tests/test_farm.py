#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.farm: a quote runs farm.py estimate --json alone, and keeps it
quoted; only one approve starts farm.py run, detached, and a second, an
unknown id or a failed quote is refused; the run's exit finishes it. A
stand-in farm.py records what it's asked. The contract: this checkout's
farm.py estimate --json, over a stand-in aws CLI and a farm of its own,
prints one JSON object with Run.estimate's keys on stdout and on stderr
the text it prints without --json, and quote reads them. Stdlib alone:

  python3 hack/simlab/tests/test_farm.py
"""

import json
import os
import re
import subprocess
import sys
import tempfile
import time
import unittest
from pathlib import Path
from unittest import mock

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import farm, suite  # noqa: E402

FARM_PY = HACK / "sim-farm" / "farm.py"
KEYS = {"jobs", "have_rows", "timed", "work_slot_h", "boot_s", "instances", "slots", "slots_by_region",
        "model_wall_s", "model_cost", "past_runs", "wall_s", "cost", "s3_cost"}

# farm.py as the tests ask it: each call's arguments (and whether it
# inherited SIMLAB_UV) appended to $FARM_CALLS; $FARM_MODE picks the answer.
STAND_IN = """\
import json, os, sys
with open(os.environ["FARM_CALLS"], "a") as f:
    f.write(json.dumps({"argv": sys.argv[1:], "uv": os.environ.get("SIMLAB_UV")}) + "\\n")
mode = os.environ.get("FARM_MODE", "ok")
if sys.argv[1] == "estimate":
    if mode == "fail":
        sys.exit("no instance type fits (families, sizes, slot_mem_gib, regions)")
    print("[12:00:00] r1: budget 256 vCPUs (quota 256, 0 in other spot instances)", file=sys.stderr)
    print("16 jobs to run (0 more have rows), about 2.8 min and $0.05", file=sys.stderr)
    print("16 jobs" if mode == "garbled" else json.dumps({"jobs": 16, "wall_s": 165, "cost": 0.05}))
elif sys.argv[1] == "run":
    print("running", flush=True)
    sys.exit(int(os.environ.get("FARM_RUN_EXIT", "0")))
"""

# The aws CLI's answers for an estimate in region r1: one zone, one
# instance type and its spot price, the quota, no other instances.
AWS = """\
#!/usr/bin/env python3
import json, sys
answer = {
    ("ec2", "describe-subnets"): {"Subnets": [{"SubnetId": "subnet-1", "VpcId": "vpc-1", "AvailabilityZone": "r1a"}]},
    ("ec2", "describe-instance-types"): {"InstanceTypes": [{"InstanceType": "c7a.16xlarge",
        "VCpuInfo": {"DefaultVCpus": 64, "DefaultCores": 32}, "MemoryInfo": {"SizeInMiB": 131072}}]},
    ("ec2", "describe-instance-type-offerings"): {"InstanceTypeOfferings": [{"InstanceType": "c7a.16xlarge",
        "Location": "r1a"}]},
    ("ec2", "describe-spot-price-history"): {"SpotPriceHistory": [{"InstanceType": "c7a.16xlarge",
        "AvailabilityZone": "r1a", "SpotPrice": "1.25"}]},
    ("service-quotas", "get-service-quota"): {"Quota": {"Value": 256}},
    ("ec2", "describe-instances"): {"Reservations": []},
}.get(tuple(sys.argv[1:3]))
if answer is None:
    sys.exit(f"An error occurred (StandIn) when calling {sys.argv[1:3]}")
print(json.dumps(answer))
"""

MATRIX = """\
package = "strims-ppspp"
target = "sim"
test = "sim_job"
slot_mem_gib = 3
[arms]
a = "HEAD"
[env]
SIM_VIEWERS = "8"
[axes]
SIM_SEED = ["2001", "2002", "2003"]
"""


def wait_for(fn, timeout_s=10.0):
    t0 = time.monotonic()
    while time.monotonic() - t0 < timeout_s:
        got = fn()
        if got:
            return got
        time.sleep(0.05)
    raise AssertionError(f"not within {timeout_s} s")


class StandIn(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-farm-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        self.farm_py = self.dir / "farm.py"
        self.farm_py.write_text(STAND_IN)
        self.calls_file = self.dir / "calls.jsonl"
        self.matrices = []
        for i, cell in enumerate(("calm", "storm")):
            m = self.dir / "matrices" / f"s0abcd-{i:02d}.toml"
            m.parent.mkdir(exist_ok=True)
            m.write_text(f"# sim-suite farm s0abcd: {cell}, why\n")
            self.matrices.append(suite.Matrix(str(m), cell, {"SIM_VIEWERS": "8"}))
        self.paths = [m.path for m in self.matrices]
        env = mock.patch.dict(os.environ, {"FARM_CALLS": str(self.calls_file), "SIMLAB_UV": "1"})
        env.start()
        self.addCleanup(env.stop)
        self.regs = farm.Regenerations(self.dir / "regenerate", farm_py=self.farm_py, python=sys.executable)

    def calls(self):
        if not self.calls_file.exists():
            return []
        return [json.loads(line) for line in self.calls_file.read_text().splitlines()]

    def runs(self):
        return [c for c in self.calls() if c["argv"][0] == "run"]

    def finished(self, id):
        r = self.regs.get(id)
        return r if r["state"] == "finished" else None

    def test_quote_runs_nothing_until_one_approve(self):
        q = farm.quote(self.matrices, self.regs)
        self.assertEqual((q.estimate, q.error), ({"jobs": 16, "wall_s": 165, "cost": 0.05}, None))
        self.assertEqual(q.text, "[12:00:00] r1: budget 256 vCPUs (quota 256, 0 in other spot instances)\n"
                                 "16 jobs to run (0 more have rows), about 2.8 min and $0.05")
        self.assertEqual(q.matrices, self.paths)
        self.assertEqual([c["argv"] for c in self.calls()], [["estimate", "--json", *self.paths]])
        r = self.regs.get(q.id)
        self.assertEqual(r["state"], "quoted")
        self.assertEqual((r["quote"], r["estimate"], r["matrices"]), (q.text, q.estimate, self.paths))
        self.assertNotIn("log_tail", r)
        # Nothing runs without the approve: not a read, another quote or the
        # server saving the request beside it.
        self.regs.save({**r, "exp_id": "e1", "cells": ["calm", "storm"], "arms": ["base", "new"],
                        "seeds": [2001, 2002], "commands": [], "warnings": ["storm: env differs"]})
        farm.quote(self.matrices[:1], self.regs)
        self.regs.get(q.id)
        time.sleep(0.3)
        self.assertEqual(self.runs(), [])

        a = farm.approve(q.id, self.regs)
        self.assertEqual(a.log, str(self.dir / "regenerate" / f"{q.id}.log"))
        r = self.regs.get(q.id)
        self.assertIn(r["state"], ("running", "finished"))
        self.assertEqual((r["pid"], r["log"], r["exp_id"], r["warnings"]), (a.pid, a.log, "e1", ["storm: env differs"]))
        self.assertTrue(r["approved_at"].endswith("Z"))
        r = wait_for(lambda: self.finished(q.id))
        self.assertEqual(r["code"], 0)
        self.assertEqual(r["log_tail"], ["running", ""])
        self.assertEqual(self.runs(), [{"argv": ["run", *self.paths], "uv": None}])
        saved = json.loads((self.dir / "regenerate" / f"{q.id}.json").read_text())
        self.assertNotIn("log_tail", saved)
        self.assertEqual(saved["state"], "finished")

    def test_a_second_approve_is_refused(self):
        q = farm.quote(self.paths, self.regs)
        farm.approve(q.id, self.regs)
        for regs in (self.regs, farm.Regenerations(self.dir / "regenerate", farm_py=self.farm_py)):
            with self.assertRaises(farm.QuoteError) as e:
                farm.approve(q.id, regs)
            self.assertEqual(e.exception.status, 409)
        wait_for(lambda: self.finished(q.id))
        with self.assertRaises(farm.QuoteError) as e:
            farm.approve(q.id, self.regs)
        self.assertEqual(e.exception.status, 409)
        self.assertEqual(len(self.runs()), 1)

    def test_a_record_quoted_again_after_its_approve_is_refused(self):
        # The log is made once: a record put back to quoted (another
        # process's stale copy) doesn't start a second run.
        q = farm.quote(self.paths, self.regs)
        stale = self.regs.get(q.id)
        farm.approve(q.id, self.regs)
        wait_for(lambda: self.finished(q.id))
        self.regs.save(stale)
        with self.assertRaises(farm.QuoteError) as e:
            farm.approve(q.id, self.regs)
        self.assertEqual(e.exception.status, 409)
        self.assertEqual(len(self.runs()), 1)

    def test_unknown_ids(self):
        for id in ("nope", "../regenerate/x", "", None):
            with self.assertRaises(farm.QuoteError) as e:
                farm.approve(id, self.regs)
            self.assertEqual(e.exception.status, 404)
            self.assertIsNone(self.regs.get(id))
        self.assertEqual(self.calls(), [])

    def test_a_failed_estimate_is_kept_and_never_runs(self):
        with mock.patch.dict(os.environ, {"FARM_MODE": "fail"}):
            q = farm.quote(self.paths, self.regs)
        self.assertIsNone(q.estimate)
        self.assertEqual(q.error, "farm.py estimate: exit 1: no instance type fits (families, sizes, "
                                  "slot_mem_gib, regions)")
        with mock.patch.dict(os.environ, {"FARM_MODE": "garbled"}):
            g = farm.quote(self.paths, self.regs)
        self.assertIsNone(g.estimate)
        self.assertRegex(g.error, "printed no JSON object: '16 jobs")
        for id in (q.id, g.id):
            self.assertEqual(self.regs.get(id)["state"], "failed")
            with self.assertRaises(farm.QuoteError) as e:
                farm.approve(id, self.regs)
            self.assertEqual(e.exception.status, 409)
        none = farm.quote([], self.regs)
        self.assertEqual((none.error, self.regs.get(none.id)["state"]), ("no matrices to quote", "failed"))
        self.assertEqual(self.runs(), [])

    def test_a_run_ends_with_its_code(self):
        q = farm.quote(self.paths, self.regs)
        with mock.patch.dict(os.environ, {"FARM_RUN_EXIT": "3"}):
            farm.approve(q.id, self.regs)
        self.assertEqual(wait_for(lambda: self.finished(q.id))["code"], 3)

    def test_a_run_another_process_started_finishes_once_it_has_gone(self):
        q = farm.quote(self.paths, self.regs)
        p = subprocess.Popen([sys.executable, "-c", "pass"])
        p.wait()
        self.regs.save({**self.regs.get(q.id), "state": "running", "pid": p.pid})
        r = self.regs.get(q.id)
        self.assertEqual((r["state"], r["code"]), ("finished", None))
        self.regs.save({**r, "state": "running", "pid": os.getpid(), "code": 0})
        self.assertEqual(self.regs.get(q.id)["state"], "running")

    def test_ids_are_twelve_url_safe_characters(self):
        ids = {farm.quote(self.paths, self.regs).id for _ in range(3)}
        self.assertEqual(len(ids), 3)
        for id in ids:
            self.assertRegex(id, r"^[A-Za-z0-9_-]{12}$")
            self.assertRegex(self.regs.get(id)["created_at"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d\.\d{3}Z$")


class Contract(unittest.TestCase):
    """This checkout's farm.py estimate --json over a farm of its own: a
    git repository as its main checkout (GIT_DIR), a matrix there and a
    stand-in aws CLI."""

    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory(prefix="simlab-farm-contract-")
        cls.dir = Path(cls.tmp.name)
        (cls.dir / "bin").mkdir()
        (cls.dir / "bin" / "aws").write_text(AWS)
        (cls.dir / "bin" / "aws").chmod(0o755)
        cls.matrix = cls.dir / "m-00.toml"
        cls.matrix.write_text(MATRIX)
        env = {k: v for k, v in os.environ.items() if not k.startswith(("AWS_", "GIT_"))}
        cls.env = env | {"GIT_DIR": str(cls.dir / ".git"), "GIT_CONFIG_NOSYSTEM": "1", "HOME": str(cls.dir),
                         "PATH": f"{cls.dir / 'bin'}:{env['PATH']}"}
        for cmd in (["init", "-q"], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "--allow-empty",
                                     "-m", "x"]):
            subprocess.run(["git", *cmd], cwd=cls.dir, env=cls.env, check=True)

    @classmethod
    def tearDownClass(cls):
        cls.tmp.cleanup()

    def estimate(self, *flags):
        return subprocess.run([sys.executable, str(FARM_PY), "estimate", *flags, "--regions", "r1", str(self.matrix)],
                              capture_output=True, text=True, env=self.env, cwd=self.dir, timeout=120)

    @staticmethod
    def untimed(text):
        return re.sub(r"(?m)^\[\d\d:\d\d:\d\d\] ", "[T] ", text)

    def test_estimate_json(self):
        p = self.estimate("--json")
        self.assertEqual(p.returncode, 0, p.stderr)
        est = json.loads(p.stdout)
        self.assertEqual(set(est), KEYS)
        self.assertEqual((est["jobs"], est["instances"], est["slots_by_region"]), (5, 1, {"r1": 32}))
        self.assertEqual(p.stdout.count("\n"), 1)
        # The text goes to stderr: what estimate prints without --json.
        text = self.estimate()
        self.assertEqual(text.returncode, 0, text.stderr)
        self.assertEqual(self.untimed(p.stderr), self.untimed(text.stdout))
        self.assertIn("5 jobs to run (0 more have rows)", p.stderr)

    def test_quote_reads_it(self):
        regs = farm.Regenerations(self.dir / "regenerate", farm_py=FARM_PY, python=sys.executable)
        # The config's regions or the aws CLI's one: here AWS_REGION's.
        with mock.patch.dict(os.environ, self.env | {"AWS_REGION": "r1"}, clear=True):
            q = farm.quote([self.matrix], regs)
        self.assertIsNone(q.error, q.text)
        self.assertEqual(set(q.estimate), KEYS)
        self.assertEqual(q.estimate["cost"], json.loads(self.estimate("--json").stdout)["cost"])
        self.assertRegex(q.text, r"^\[\d\d:\d\d:\d\d\] r1: budget 256 vCPUs")
        self.assertEqual(regs.get(q.id)["state"], "quoted")


if __name__ == "__main__":
    unittest.main()
