#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.farm: a quote runs farm.py estimate --json alone, and keeps it
quoted; only one approve starts farm.py run, detached, and a second, an
unknown id or a failed quote is refused; the run's exit finishes it. A
stand-in farm.py records what it's asked. The contract: this checkout's
farm.py estimate --json, over a stand-in aws CLI and a farm of its own,
prints one JSON object with Run.estimate's keys on stdout and on stderr
the text it prints without --json, and quote reads them.

Live runs, this checkout's farm.py over a farm of its own (FARM): a run
is live while a process holds its lock, which makes no state dir (a run
whose build fails leaves none), and a second run of it is refused before
its builds; stop resolves its target to the live runs that are it or
include it, refuses none or several and a lock's PID the run didn't start,
and sends the one a single SIGINT, which a run takes from its start though
started with SIGINT ignored (in its builds it exits at once; once its
setup starts a host, through its cleanup), then prints its log until it
exits and lists the instances left with its exact tag; status lists the
live runs, and down --run terminates one run's instances alone. Stand-in
runs are processes named farm.py that hold the lock with farm.py's own
hold_run, or run its main (its builds stubbed, or its own over a binary
in place) with a stand-in --local worker.

Shared keys, this checkout's Run over a farm of its own: the jobs of one
key under several ids (arms whose binaries are the same bytes) run once,
as the run's first of them, and each gets its own line of that run, all
but the first's with `from`; a derived job waits on the job that runs its
source's key, and every job of its key takes what that took; the verify
copies are of as many keys, and count by job, as a rerun's lines do; a
job whose binary isn't built shares nothing. A testbed run's tables, a
stand-in testbed_tables', fold into a host's store as a test job's out
dir does, under its tables' key, sim, test and env, its cell, arm and
repeat, its row with the host's, and the flush lands in its sink; a
testbed_tables that fails leaves no tables and says why; the controller
writes a stored testbed run's line once its flush lands, naming its key
and source, and an unstored one's at once. A testbed run's estimate
prices a host per group of the cheapest type, its jobs timed by their
matrix's earlier runs or by run.sh's timing, and a set of two host_cores
is refused. Stdlib alone:

  python3 hack/simlab/tests/test_farm.py
"""

import collections
import contextlib
import importlib.util
import io
import json
import os
import re
import signal
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


def load_farm_py():
    """This checkout's hack/sim-farm/farm.py as a module of its own (it
    imports worker beside it). Its FARM is the main checkout's: each test
    points it at a directory of its own."""
    if str(FARM_PY.parent) not in sys.path:
        sys.path.insert(0, str(FARM_PY.parent))
    spec = importlib.util.spec_from_file_location("sim_farm_farm", FARM_PY)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


fp = load_farm_py()
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


# A live run as stop sees one: a process named farm.py, run as `farm.py run
# M...`, that takes the run's lock with this checkout's hold_run in
# $STAND_IN_FARM, logs to its first matrix's farm.log, records each signal
# in $STAND_IN_OUT/signals as it comes, and after a SIGINT logs its cleanup
# ($STAND_IN_LINES, |-separated) over $STAND_IN_LINGER s and exits.
STAND_IN_RUN = """\
import importlib.util, os, signal, sys, time
from pathlib import Path
real = Path(os.environ["REAL_FARM_PY"])
sys.path.insert(0, str(real.parent))
spec = importlib.util.spec_from_file_location("farm_under_test", real)
fp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fp)
fp.FARM = Path(os.environ["STAND_IN_FARM"])
out = Path(os.environ["STAND_IN_OUT"])
ms = [a for a in sys.argv[sys.argv.index("run") + 1:] if not a.startswith("--")]
got = []

def on(sig, _):
    got.append(sig)
    with (out / "signals").open("a") as f:
        f.write(signal.Signals(sig).name + "\\n")

signal.signal(signal.SIGINT, on)
fp.hold_run("+".join(sorted(ms)), ms)
log = fp.FARM / ms[0] / "farm.log"
log.parent.mkdir(parents=True, exist_ok=True)

def say(msg):
    with log.open("a") as f:
        f.write(f"[00:00:00] {msg}\\n")

say("running")
deadline = time.monotonic() + 30
while not got and time.monotonic() < deadline:
    time.sleep(0.02)
if not got:
    sys.exit(1)
say("interrupted")
time.sleep(float(os.environ.get("STAND_IN_LINGER", "0.6")))
for line in filter(None, os.environ.get("STAND_IN_LINES", "").split("|")):
    say(line)
time.sleep(0.2)
sys.exit(130)
"""

# farm.py's own main over the test's farm: GIT_DIR makes the test's
# repository the main checkout, so every path of farm.py's state and config
# is the test's. $STAND_IN_BUILD picks the builds: "wait" (record the build,
# wait 10 s for a signal, fail), "fail" (record it, fail at once) or "real"
# (farm.py's own, of a binary put in place, which it keeps). The run leaves
# out the DuckDB CLI, and its --local worker is a stand-in that records a
# SIGTERM; once the worker is up the setup waits for a signal (10 s at
# most), which then comes as the setup starts a host.
RUN_DRIVER = """\
import importlib.util, os, subprocess, sys, time
from pathlib import Path
real = Path(os.environ["REAL_FARM_PY"])
sys.path.insert(0, str(real.parent))
spec = importlib.util.spec_from_file_location("farm_under_test", real)
fp = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fp)
assert fp.FARM == Path(os.environ["STAND_IN_FARM"]), fp.FARM
out = Path(os.environ["STAND_IN_OUT"])
mode = os.environ.get("STAND_IN_BUILD", "wait")
WORKER = '''
import signal, sys, time
from pathlib import Path
out = Path(sys.argv[1])

def term(*_):
    (out / "worker-terminated").write_text("SIGTERM")
    sys.exit(0)

signal.signal(signal.SIGTERM, term)
(out / "worker-ready").write_text("")
time.sleep(60)
'''

def build(m):
    with (out / "built").open("a") as f:
        f.write(f"{os.getpid()} {m.name}\\n")
    if mode == "fail":
        sys.exit("the build stub fails")
    time.sleep(10)
    sys.exit("the build stub's wait ran out: no SIGINT came")

def start_local(self, h):
    w = subprocess.Popen([sys.executable, "-c", WORKER, str(out)])
    self.serves.append(w)
    while not (out / "worker-ready").exists():
        time.sleep(0.02)
    with (out / "launched").open("a") as f:
        f.write(f"{w.pid}\\n")
    deadline = time.monotonic() + 10
    while not self.deferred and time.monotonic() < deadline:
        time.sleep(0.02)

if mode == "real":
    for arg in sys.argv[2:]:
        if arg.endswith(".toml"):
            m = fp.Matrix(arg)
            m.resolve()
            fp.BIN.mkdir(parents=True, exist_ok=True)
            for source in m.sources.values():
                (fp.BIN / m.bin_name(source)).write_text("#!/bin/sh\\n")
else:
    fp.build = build
fp.Run.ship_duckdb = lambda self: None
fp.Run.start_local = start_local
sys.exit(fp.main())
"""


class Live(unittest.TestCase):
    """farm.py's live runs, stop and down --run, over a FARM of the test's
    own; the aws CLI is never called (farm_instances is a stand-in)."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-farm-live-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        self.farm = self.dir / "target" / "sim-farm"
        self.farm.mkdir(parents=True)
        for name, value in (("FARM", self.farm), ("sweep_regions", lambda rs: sorted(set(rs))),
                            ("farm_instances", self.no_aws)):
            p = mock.patch.object(fp, name, value)
            p.start()
            self.addCleanup(p.stop)
        self.n = 0

    @staticmethod
    def no_aws(region, run):
        raise AssertionError(f"an aws call in {region} for {run}")

    def stand_in(self, *argv, script=STAND_IN_RUN, ignore_sigint=False, base=None, **env):
        """A stand-in farm.py run of argv, and the directory it records in."""
        self.n += 1
        d = self.dir / f"stand-in-{self.n}"
        d.mkdir()
        (d / "farm.py").write_text(script)
        p = subprocess.Popen(
            [sys.executable, str(d / "farm.py"), "run", *argv],
            env=(base or os.environ) | {"REAL_FARM_PY": str(FARM_PY), "STAND_IN_FARM": str(self.farm),
                                        "STAND_IN_OUT": str(d), **env},
            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True,
            preexec_fn=(lambda: signal.signal(signal.SIGINT, signal.SIG_IGN)) if ignore_sigint else None,
        )
        self.addCleanup(p.stderr.close)
        self.addCleanup(lambda: p.poll() is None and (p.kill(), p.wait()))
        return p, d

    def driven(self, *flags, **env):
        """RUN_DRIVER's `farm.py run` of a matrix of the test's own, over a
        git repository of the test's own (one empty commit, HEAD the
        matrix's arm) as the main checkout, and the directory it records in."""
        base = {k: v for k, v in os.environ.items() if not k.startswith(("AWS_", "GIT_"))}
        base |= {"GIT_DIR": str(self.dir / ".git"), "GIT_CONFIG_NOSYSTEM": "1", "HOME": str(self.dir),
                 "SIM_DB_ROOT": str(self.dir / "sim-db")}
        if not (self.dir / ".git").exists():
            for cmd in (["init", "-q"], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q",
                                         "--allow-empty", "-m", "x"]):
                subprocess.run(["git", *cmd], cwd=self.dir, env=base, check=True)
            (self.dir / "m-00.toml").write_text(MATRIX)
        return self.stand_in(str(self.dir / "m-00.toml"), "--local", *flags, script=RUN_DRIVER, base=base, **env)

    def worker_gone(self, d):
        """Kills the stand-in workers a driven run launched that are still up."""
        def kill():
            with contextlib.suppress(OSError):
                for pid in map(int, (d / "launched").read_text().split()):
                    if fp.proc_argv(pid)[-1:] == [str(d)]:
                        with contextlib.suppress(OSError):
                            os.kill(pid, signal.SIGKILL)
        self.addCleanup(kill)

    def live(self, name, p):
        """Waits for run `name` to be live, held by p, and returns it."""
        return wait_for(lambda: next((r for r in fp.live_runs() if r.name == name and r.pid == p.pid), None))

    @staticmethod
    def signals(d):
        f = d / "signals"
        return f.read_text().split() if f.exists() else []

    @staticmethod
    def call(fn, *args, **kwargs):
        """fn's result and what it printed, in 20 s: a stop that waits on
        longer fails."""
        def late(*_):
            raise TimeoutError("not within 20 s")

        out = io.StringIO()
        was = signal.signal(signal.SIGALRM, late)
        signal.alarm(20)
        try:
            with contextlib.redirect_stdout(out):
                got = fn(*args, **kwargs)
        finally:
            signal.alarm(0)
            signal.signal(signal.SIGALRM, was)
        return got, out.getvalue()

    def test_a_run_is_live_while_a_process_holds_its_lock(self):
        fd = fp.hold_run("m1+m2", ["m1", "m2"])
        [r] = fp.live_runs()
        self.assertEqual((r.name, r.pid, r.start, r.checkout, r.matrices),
                         ("m1+m2", os.getpid(), fp.proc_start(os.getpid()), str(fp.ROOT), ["m1", "m2"]))
        self.assertEqual(r.lock, self.farm / "live" / "m1+m2.pid")
        # The lock makes no state dir: those are the run's loop's.
        self.assertEqual(os.listdir(self.farm), ["live"])
        os.close(fd)
        # The file stays, naming this process, alive and started then: a
        # lock no process holds is a run that ended.
        self.assertEqual(json.loads(r.lock.read_text())["pid"], os.getpid())
        self.assertEqual(fp.live_runs(), [])
        self.assertEqual(self.call(fp.stop, "m1", lambda: []), (1, "no live run is or includes m1\n"))

    def test_a_target_resolves_to_the_live_runs_that_are_it_or_include_it(self):
        for name, ms in (("a+b", ["a", "b"]), ("b+c", ["b", "c"]), ("d", ["d"]),
                         ("f+2more-0123abcd", ["f", "g", "h"])):
            self.addCleanup(os.close, fp.hold_run(name, ms))
        os.close(fp.hold_run("e", ["e"]))
        runs = fp.live_runs()
        self.assertEqual([r.name for r in runs], ["a+b", "b+c", "d", "f+2more-0123abcd"])
        for target, want in (("a+b", ["a+b"]), ("a", ["a+b"]), ("b", ["a+b", "b+c"]),
                             ("hack/sim-farm/matrices/c.toml", ["b+c"]), ("c.toml", ["b+c"]), ("d", ["d"]),
                             ("g", ["f+2more-0123abcd"]), ("e", []), ("a+c", []), ("x", [])):
            self.assertEqual([r.name for r in fp.live_of(target, runs)], want, target)

    def test_stop_refuses_a_matrix_two_live_runs_include(self):
        p1, d1 = self.stand_in("a", "b", "--local")
        p2, d2 = self.stand_in("c", "b", "--local")
        r1, r2 = self.live("a+b", p1), self.live("b+c", p2)
        code, out = self.call(fp.stop, "b", lambda: [])
        self.assertEqual(code, 2)
        self.assertEqual(out, f"2 live runs include b; stopping none. Name the run to stop:\n"
                              f"  a+b  pid {p1.pid}  started {r1.started}  from {fp.ROOT}\n"
                              f"  b+c  pid {p2.pid}  started {r2.started}  from {fp.ROOT}\n")
        time.sleep(0.3)
        self.assertEqual((self.signals(d1), self.signals(d2)), ([], []))
        # Each by its name.
        for name, p, d in (("a+b", p1, d1), ("b+c", p2, d2)):
            self.assertEqual(self.call(fp.stop, name, lambda: [])[0], 0)
            self.assertEqual((p.wait(5), self.signals(d)), (130, ["SIGINT"]))
        self.assertEqual(fp.live_runs(), [])

    def test_stop_refuses_a_pid_the_run_did_not_start(self):
        p, d = self.stand_in("m1", "--local")
        r = self.live("m1", p)
        rec = json.loads(r.lock.read_text())
        dead = subprocess.Popen([sys.executable, "-c", "pass"])
        dead.wait()
        other = subprocess.Popen(["sleep", "60"])
        self.addCleanup(lambda: (other.kill(), other.wait()))
        wait_for(lambda: fp.proc_argv(other.pid) == ["sleep", "60"])
        # The file rewritten in place, its lock still held: its PID given
        # since to another farm.py run, to another process, or to none.
        for pid, start, why in ((p.pid, rec["start"] + 1, f"PID {p.pid} started at another time than the run's "
                                                          "process: another process, given its PID"),
                                (other.pid, fp.proc_start(other.pid), f"PID {other.pid} isn't a farm.py run: sleep 60"),
                                (dead.pid, rec["start"], f"PID {dead.pid} isn't running")):
            r.lock.write_text(json.dumps({**rec, "pid": pid, "start": start}) + "\n")
            code, out = self.call(fp.stop, "m1", lambda: [])
            self.assertEqual(code, 1)
            self.assertTrue(out.startswith(f"m1: not signalled: {why}"), out)
        time.sleep(0.3)
        self.assertEqual(self.signals(d), [])
        self.assertEqual((p.poll(), other.poll()), (None, None))

    def test_a_second_run_of_a_live_run_is_refused(self):
        p, d = self.stand_in("m1", "m2", "--local")
        r = self.live("m1+m2", p)
        with self.assertRaises(SystemExit) as e:
            fp.hold_run("m1+m2", ["m1", "m2"])
        self.assertRegex(str(e.exception.code), rf"^m1\+m2 is live, and runs once at a time: m1\+m2  pid {p.pid}  ")
        self.assertEqual(fp.holder(r.lock), r)
        # A run that shares a matrix is another run.
        os.close(fp.hold_run("m1", ["m1"]))
        self.assertEqual(self.signals(d), [])

    def test_a_run_takes_its_lock_before_its_builds_and_sigint_from_its_start(self):
        # Started where SIGINT is ignored, as a non-interactive shell's
        # background job is.
        p, d = self.driven(ignore_sigint=True)
        wait_for(lambda: (d / "built").exists())
        self.live("m-00", p)
        p2, d2 = self.driven()
        _, err = p2.communicate(timeout=30)
        self.assertEqual(p2.returncode, 1)
        self.assertRegex(err, rf"m-00 is live, and runs once at a time: m-00  pid {p.pid}  ")
        self.assertFalse((d2 / "built").exists())
        # A signal in the builds stops the run at once: it has started no
        # host, and its loop no state dir.
        code, out = self.call(fp.stop, "m-00", lambda: [])
        self.assertEqual(code, 0, out)
        self.assertEqual(p.wait(5), 130)
        self.assertIn("interrupted before the run started a host: nothing to clean up", p.stderr.read())
        self.assertEqual((d / "built").read_text(), f"{p.pid} m-00\n")
        self.assertEqual(os.listdir(self.farm), ["live"])

    def test_a_run_whose_build_fails_makes_no_state_dir(self):
        p, d = self.driven(STAND_IN_BUILD="fail")
        _, err = p.communicate(timeout=30)
        self.assertEqual((p.returncode, err), (1, "the build stub fails\n"))
        self.assertEqual((d / "built").read_text(), f"{p.pid} m-00\n")
        # simlab.suite.farm_started reads a matrix's state dir as the run
        # having started: the lock, in live/, makes none.
        self.assertEqual(os.listdir(self.farm), ["live"])
        self.assertEqual(fp.live_runs(), [])

    def test_a_signal_once_the_setup_starts_a_host_goes_through_the_cleanup(self):
        # farm.py's own builds (of a binary in place), Run and its setup;
        # the --local worker a stand-in.
        p, d = self.driven(STAND_IN_BUILD="real")
        self.worker_gone(d)
        wait_for(lambda: (d / "launched").exists())
        code, out = self.call(fp.stop, "m-00", lambda: [])
        self.assertEqual(code, 0, out)
        self.assertEqual(p.wait(5), 130)
        # The cleanup stopped the worker the setup started, and logged.
        self.assertEqual((d / "worker-terminated").read_text(), "SIGTERM")
        self.assertNotIn("interrupted before", p.stderr.read())
        log = (self.farm / "m-00" / "farm.log").read_text()
        self.assertRegex(log, r"\] a signal came as the run set up its hosts; stopping\n\[[\d:]+\] interrupted\n")
        self.assertIn("a signal came as the run set up its hosts; stopping\n", out)

    def test_stop_signals_the_one_live_run_once_and_waits_for_it(self):
        p, d = self.stand_in("m1", "m2", "--local", STAND_IN_LINES="winding down|done")
        self.live("m1+m2", p)
        code, out = self.call(fp.stop, "m2", lambda: self.fail("a --local run's regions"))
        self.assertEqual(code, 0)
        # It has exited, and stop printed its log from the signal on.
        self.assertEqual((p.poll(), self.signals(d)), (130, ["SIGINT"]))
        self.assertTrue(out.startswith(f"m1+m2: sent SIGINT to farm.py run {p.pid} (started "), out)
        self.assertTrue(out.endswith(
            f"m1+m2: waiting while it cleans up; its log ({self.farm / 'm1' / 'farm.log'}):\n"
            "[00:00:00] interrupted\n[00:00:00] winding down\n[00:00:00] done\n"
            "m1+m2: exited\na --local run launches no instances\n"), out)
        self.assertEqual(fp.live_runs(), [])

    def test_stop_lists_the_instances_left_with_the_runs_exact_tag(self):
        p, d = self.stand_in("m1", STAND_IN_LINES="leaving 1 interrupted instances for AWS to reclaim: i-2")
        self.live("m1", p)
        asked = []

        def instances(region, run):
            asked.append((region, run))
            return [{"InstanceId": iid, "State": {"Name": state}, "Tags": [{"Key": fp.TAG, "Value": tag}]}
                    for iid, state, tag in (("i-1", "running", "m1"), ("i-2", "running", "m1"),
                                            ("i-3", "shutting-down", "m1"), ("i-9", "running", "m1+m9"))]

        with mock.patch.object(fp, "farm_instances", instances):
            code, out = self.call(fp.stop, "m1", lambda: ["r1"])
        self.assertEqual((code, asked, self.signals(d)), (3, [("r1", "m1")], ["SIGINT"]))
        self.assertTrue(out.endswith(
            "m1: exited\n  i-1  r1  running\n  i-2  r1  running  interrupted: AWS reclaims it\n"
            "1 instances tagged m1 are left: `python3 hack/sim-farm/farm.py down --run m1` terminates them, "
            "the interrupted ones too\n"), out)

        p, d = self.stand_in("m1")
        self.live("m1", p)
        with mock.patch.object(fp, "farm_instances", lambda region, run: []):
            code, out = self.call(fp.stop, "m1", lambda: ["r1"])
        self.assertEqual(code, 0)
        self.assertTrue(out.endswith("m1: exited\nno instance tagged m1 is left\n"), out)

    def test_no_wait_returns_while_the_run_cleans_up(self):
        p, d = self.stand_in("m1", "--local", STAND_IN_LINGER="2")
        self.live("m1", p)
        code, out = self.call(fp.stop, "m1", lambda: [], wait=False)
        self.assertEqual(code, 0)
        self.assertIn("m1: not waiting; its cleanup goes on", out)
        wait_for(lambda: self.signals(d))
        self.assertEqual([r.name for r in fp.live_runs()], ["m1"])
        self.assertEqual(p.wait(10), 130)
        self.assertEqual((self.signals(d), fp.live_runs()), (["SIGINT"], []))

    def test_status_lists_the_live_runs(self):
        self.assertEqual(self.call(fp.status, None, []), (0, "no live runs\n"))
        p, _ = self.stand_in("m1", "m2", "--local")
        r = self.live("m1+m2", p)
        self.assertEqual(self.call(fp.status, None, []),
                         (0, f"live runs:\n  m1+m2  pid {p.pid}  started {r.started}  from {fp.ROOT}\n"))
        self.assertRegex(r.started, r"^\d{4}-\d\d-\d\d \d\d:\d\d:\d\d ")
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            fp.say_live(fp.live_of("m2", fp.live_runs()), "m2")
            fp.say_live(fp.live_of("m3", fp.live_runs()), "m3")
        self.assertEqual(out.getvalue(), f"live runs that include m2:\n  {r.line()}\nno live run includes m3\n")

    def test_down_run_terminates_the_run_of_that_name_alone(self):
        tags = {"i-1": "m1", "i-2": "m1+m2", "i-3": "m2"}
        asked, gone = [], []

        # Every instance, whatever tag the listing asks for (a tag filter's
        # values take wildcards): down holds each to the run's name too.
        def instances(region, run):
            asked.append(run)
            return [{"InstanceId": i, "State": {"Name": "running"}, "Tags": [{"Key": fp.TAG, "Value": t}]}
                    for i, t in tags.items()]

        with mock.patch.object(fp, "farm_instances", instances), \
                mock.patch.object(fp, "terminate", lambda region, ids: gone.append(sorted(ids))):
            self.assertEqual(self.call(fp.down, "m1", ["r1"], exact=True), (0, "r1: terminating 1 instances\n"))
            self.assertEqual(self.call(fp.down, "m1", ["r1"]), (0, "r1: terminating 2 instances\n"))
        self.assertEqual((asked, gone), (["m1", None], [["i-1"], ["i-1", "i-2"]]))


def farm_of(repo):
    """This checkout's farm.py as a module whose main checkout is the git
    repository `repo` (GIT_DIR, which its git calls read too: set it while
    the module runs), so its state, binaries and config are repo's."""
    with mock.patch.dict(os.environ, {"GIT_DIR": str(repo / ".git")}):
        spec = importlib.util.spec_from_file_location("sim_farm_farm_keys", FARM_PY)
        mod = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(mod)
    return mod


# A sweep of three seeds whose X=b jobs wait on their X=a jobs and take
# STRIMS_SIM_Y from that row's frames.
KEYED = """\
package = "strims-sim"
target = "scenarios"
test = "sweep"
[arms]
{arms}
[env]
STRIMS_SIM_JOB = "1"
[axes]
STRIMS_SIM_SEED = ["1", "2", "3"]
STRIMS_SIM_X = ["a", "b"]
[[derive]]
when = {{ STRIMS_SIM_X = "b" }}
from = {{ STRIMS_SIM_X = "a" }}
set = {{ STRIMS_SIM_Y = "frames" }}
[report]
columns = ["{column}"]
"""


class SharedKeys(unittest.TestCase):
    """Jobs of one key under several ids: arms old and new of k-00 and arm
    other of k-01 build from commits whose sources differ (crates/x) into
    binaries of the same bytes, so a job of each arm shares its key with
    the others' of its environment. The run's first job of a key runs for
    all of them: each gets its own line, of the flush that one run landed
    in, all but the first's with `from` naming k-00; what a derived job
    waits on, takes and keeps, the verify copies, the binaries shipped and
    the priors follow the job that runs."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-farm-keys-")
        self.addCleanup(tmp.cleanup)
        self.dir = d = Path(tmp.name)
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        env |= {"GIT_DIR": str(d / ".git"), "GIT_WORK_TREE": str(d), "GIT_CONFIG_NOSYSTEM": "1",
                "HOME": str(d), "SIM_DB_ROOT": str(d / "sim-db")}
        p = mock.patch.dict(os.environ, env, clear=True)
        p.start()
        self.addCleanup(p.stop)
        subprocess.run(["git", "init", "-q"], cwd=d, check=True)
        (d / "crates").mkdir()
        self.commits = {}
        for arm in ("old", "new", "other"):
            (d / "crates" / "x").write_text(arm)
            for cmd in (["add", "crates"], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", arm]):
                subprocess.run(["git", *cmd], cwd=d, check=True)
            self.commits[arm] = subprocess.run(["git", "rev-parse", "HEAD"], cwd=d, check=True,
                                               capture_output=True, text=True).stdout.strip()
        self.fp = farm_of(d)
        self.assertEqual(self.fp.FARM, d / "target" / "sim-farm")
        self.path = {}
        for name, arms, column in (("k-00", ("old", "new"), "delivery:mean"),
                                   ("k-01", ("other",), "lag_p50_ms:median"),
                                   ("k-02", ("old", "new"), "delivery:mean")):
            self.path[name] = d / f"{name}.toml"
            self.path[name].write_text(KEYED.format(
                arms="\n".join(f'{a} = "{self.commits[a]}"' for a in arms), column=column))
        self.fp.BIN.mkdir(parents=True)
        self.ms = [self.matrix("k-00"), self.matrix("k-01")]
        self.bins = sorted({m.bin_name(s) for m in self.ms for s in m.sources.values()})
        self.assertEqual(len(self.bins), 3)
        for b in self.bins:
            (self.fp.BIN / b).write_text("#!/bin/sh\n")
        self.host = self.fp.Host("local", "local", "-", self.fp.Spec(2, 2, 0, 2), 0.0, time.time(), state="ready")

    def matrix(self, name):
        m = self.fp.Matrix(str(self.path[name]))
        m.resolve()
        return m

    def run_of(self, ms, verify=2):
        """The run of `ms` as Run.run starts it: its matrices' state dirs
        made, the rows of earlier runs copied in, and what waits on a row
        it has queued."""
        args = mock.Mock(local=True, s3_region=None, s3_bucket=None, verify=verify)
        with contextlib.redirect_stdout(io.StringIO()):
            run = self.fp.Run(ms, args)
            for m in ms:
                m.state.mkdir(parents=True, exist_ok=True)
            run.share_rows()
            for jid in [j for j in run.waiting if j in run.rows]:
                run.release(jid)
        return run

    @staticmethod
    def todo(run, copy=""):
        return [i for i in run.queue if i.copy == copy and i.wid not in run.done]

    def lines(self, name):
        return self.fp.read_jsonl(self.fp.FARM / name / self.fp.INDEX)

    def finish(self, run, items, seq):
        """Stores a row for each of `items` in flush `seq` of the host, its
        frames from its seed (a derived job's from its own), and lands it."""
        for item in items:
            env = run.env_of(item.job)
            frames = int(env["STRIMS_SIM_SEED"]) * 10 + (env["STRIMS_SIM_X"] == "b")
            run.store(self.host, item, {"row": {"trace": f"t{frames}", "frames": frames}, "wall_s": 2.0 + frames,
                                        "cpu_s": 1.0 + frames, "max_rss_mb": 100 + frames, "flush": seq})
        run.land(self.host, seq, {"reads": True})
        run.flush_rows()

    def test_one_run_of_a_key_gives_every_job_of_it_its_own_line(self):
        jobs = {m.name: m.jobs() for m in self.ms}
        every = [j for js in jobs.values() for j in js]
        self.assertEqual((len(every), len({j.id for j in every}), len({j.key for j in every})), (18, 18, 6))
        run = self.run_of(self.ms)
        old = {j.id: j for j in jobs["k-00"] if j.arm == "old"}
        # The first matrix's first arm's jobs run, each for its key's jobs.
        self.assertEqual({i.job.id for i in run.items.values() if not i.copy}, set(old))
        self.assertEqual(set(run.owners), set(old))
        fields = list(dict.fromkeys(self.ms[0].row_fields() + self.ms[1].row_fields()))
        for jid, own in run.owners.items():
            self.assertEqual([(m.name, j.arm) for m, j in own], [("k-00", "old"), ("k-00", "new"), ("k-01", "other")])
            self.assertEqual({j.key for _, j in own}, {old[jid].key})
            self.assertEqual({run.lead[j.id] for _, j in own}, {jid})
            # Its line keeps both matrices' fields.
            self.assertEqual(run.fields[jid], fields)
        self.assertEqual(run.bins, [self.ms[0].bin_name(self.ms[0].sources["old"])])
        # A derived job waits on the job that runs its source's key.
        plain = {j.id for j in old.values() if not j.after}
        derived = [j for j in old.values() if j.after]
        self.assertEqual({i.job.id for i in self.todo(run)}, plain)
        self.assertEqual(set(run.waiting), plain)
        self.assertEqual({w.job.after for ws in run.waiting.values() for w in ws}, plain)
        # The verify copies: each matrix's first jobs by id, of two keys.
        verify = {i.job.id for i in run.items.values() if i.copy == "v"}
        for m in self.ms:
            first = list(dict.fromkeys(run.lead[j.id] for j in sorted(jobs[m.name], key=lambda j: j.id)))
            self.assertLessEqual(set(first[:2]), verify, m.name)
        self.assertLessEqual(len(verify), 4)

        self.finish(run, self.todo(run), 0)
        # What a derived job takes is its item's, and so its owners' too.
        self.assertEqual({i.job.id for i in self.todo(run)}, {j.id for j in derived})
        for j in derived:
            seed = int(dict(j.env)["STRIMS_SIM_SEED"])
            self.assertEqual(run.derived[j.id], {"STRIMS_SIM_Y": str(seed * 10)})
            for _, o in run.owners[j.id]:
                self.assertEqual(run.env_of(o)["STRIMS_SIM_Y"], str(seed * 10))
        self.finish(run, self.todo(run), 1)
        self.assertEqual((self.todo(run), run.waiting), ([], {}))

        lines = {n: self.lines(n) for n in ("k-00", "k-01")}
        self.assertEqual([len(v) for v in lines.values()], [12, 6])
        self.assertEqual({r["id"] for v in lines.values() for r in v}, {j.id for j in every})
        by_key = collections.defaultdict(list)
        for n, v in lines.items():
            for r in v:
                by_key[r["key"]].append((n, r))
        self.assertEqual(len(by_key), 6)
        same = ("binary", "env", "host", "wall_s", "cpu_s", "max_rss_mb", "src", "data", "reads", "row")
        for key, got in by_key.items():
            self.assertEqual([(n, r["arm"], r.get("from")) for n, r in got],
                             [("k-00", "old", None), ("k-00", "new", "k-00"), ("k-01", "other", "k-00")])
            self.assertEqual(len({json.dumps([r[f] for f in same]) for _, r in got}), 1, key)
            for _, r in got:
                self.assertEqual(r["commit"], self.commits[r["arm"]])
                if r["env"]["STRIMS_SIM_X"] == "b":
                    self.assertEqual(r["env"]["STRIMS_SIM_Y"], str(int(r["env"]["STRIMS_SIM_SEED"]) * 10))
        # The run ran each key once, and walls count it once.
        self.assertEqual(run.ran, 6)
        self.assertEqual(len(self.fp.index_walls()), 6)

    def test_a_rerun_takes_each_jobs_line_and_verifies_by_job(self):
        first = self.run_of(self.ms)
        self.finish(first, self.todo(first), 0)
        self.finish(first, self.todo(first), 1)
        # Every job has a line of its own and none of a verify copy, though
        # a key has three lines: each verify copy runs.
        run = self.run_of(self.ms)
        verify = self.todo(run, "v")
        self.assertTrue(verify)
        self.assertEqual({i.wid for i in run.items.values() if i.wid not in run.done}, {i.wid for i in verify})
        self.assertEqual(set(run.prior), set(run.owners))
        for m in self.ms:
            self.assertEqual(len(self.lines(m.name)), len(m.jobs()), m.name)
        self.finish(run, verify, 2)
        self.assertEqual((run.verified, run.mismatches), (len(verify), 0))
        for n, owners in (("k-00", 2), ("k-01", 1)):
            copies = [r for r in self.lines(n) if r["copy"] == "v"]
            self.assertEqual(len(copies), len(verify) * owners, n)
        again = self.run_of(self.ms)
        self.assertEqual(again.done, set(again.items))
        # A matrix new to the farm takes a line of each of its jobs, of its
        # own id, arm and commit, from a matrix that ran its key.
        m2 = self.matrix("k-02")
        late = self.run_of([m2])
        self.assertEqual({i.copy for i in late.items.values() if i.wid not in late.done}, {"v"})
        got = self.lines("k-02")
        self.assertEqual(sorted((r["id"], r["arm"], r["commit"]) for r in got),
                         sorted((j.id, j.arm, j.commit) for j in m2.jobs()))
        self.assertEqual({r.get("from") for r in got}, {"k-00"})

    def test_a_verify_copy_whose_trace_differs_is_a_mismatch(self):
        first = self.run_of(self.ms)
        self.finish(first, self.todo(first), 0)
        self.finish(first, self.todo(first), 1)
        run = self.run_of(self.ms)
        verify = self.todo(run, "v")
        self.assertTrue(verify)
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            for item in verify:
                run.store(self.host, item, {"row": {"trace": "t-other", "reads_hash": "r2"}, "wall_s": 1.0,
                                            "flush": 2})
        self.assertEqual((run.verified, run.mismatches), (0, len(verify)))
        said = [line for line in out.getvalue().splitlines() if "MISMATCH" in line]
        self.assertEqual(len(said), len(verify))
        for line, item in zip(said, verify):
            self.assertIn(f"MISMATCH: job {item.job.id} ", line)
            self.assertIn(f"trace {run.rows[item.job.id]['row']['trace']} reads None on local, trace t-other reads r2 "
                          "on local", line)
        got = self.fp.read_jsonl(self.fp.FARM / "k-00" / "mismatches.jsonl")
        self.assertEqual({r["again"]["row"]["trace"] for r in got}, {"t-other"})

    def test_a_job_whose_binary_isnt_built_runs_as_its_own(self):
        # An estimate's: its key is unknown.
        for b in self.bins[1:]:
            (self.fp.BIN / b).unlink()
        ms = [self.matrix("k-00"), self.matrix("k-01")]
        self.assertEqual(len([j for m in ms for j in m.jobs() if j.key]), 6)
        run = self.run_of(ms, verify=0)
        self.assertEqual(len(run.items), 18)
        self.assertTrue(all(len(own) == 1 for own in run.owners.values()))


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

    def test_status_and_stop_from_the_command_line(self):
        d = self.dir / "stand-in"
        d.mkdir()
        (d / "farm.py").write_text(STAND_IN_RUN)
        p = subprocess.Popen([sys.executable, str(d / "farm.py"), "run", "m-00"], stdout=subprocess.DEVNULL,
                             env=self.env | {"REAL_FARM_PY": str(FARM_PY), "STAND_IN_OUT": str(d),
                                             "STAND_IN_FARM": str(self.dir / "target" / "sim-farm")})
        self.addCleanup(lambda: p.poll() is None and p.kill())

        def farm(*args):
            return subprocess.run([sys.executable, str(FARM_PY), *args], capture_output=True, text=True,
                                  env=self.env, cwd=self.dir, timeout=60)

        got = wait_for(lambda: (s := farm("status", "--regions", "r1")).stdout.startswith("live runs:") and s)
        self.assertEqual(got.returncode, 0, got.stderr)
        self.assertRegex(got.stdout, rf"^live runs:\n  m-00  pid {p.pid}  started [^\n]+  from {FARM_PY.parents[2]}\n$")
        none = farm("stop", "nope")
        self.assertEqual((none.returncode, none.stdout), (1, "no live run is or includes nope\n"))
        s = farm("stop", str(self.matrix), "--regions", "r1")
        self.assertEqual(s.returncode, 0, s.stdout + s.stderr)
        self.assertTrue(s.stdout.endswith("m-00: exited\nno instance tagged m-00 is left\n"), s.stdout)
        self.assertEqual((p.wait(5), (d / "signals").read_text()), (130, "SIGINT\n"))
        self.assertEqual(farm("status", "--regions", "r1").stdout, "no live runs\n")

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


# A stand-in testbed_tables: <out> <tables> [--knob K=V ...] writes one
# table, the viewers', series_meta.json and job.json, its env the knobs.
FAKE_TABLES = """\
#!/usr/bin/env python3
import json, sys
from pathlib import Path
out, dest = Path(sys.argv[1]), Path(sys.argv[2])
knobs = dict(sys.argv[i + 1].split("=", 1) for i in range(3, len(sys.argv), 2))
if (out / "fail").exists():
    sys.exit("no node lines")
dest.mkdir()
(dest / "viewers.csv").write_text("viewer,host\\n0,1\\n1,2\\n")
(dest / "viewers.schema.json").write_text('{"columns":[{"name":"viewer","type":"BIGINT"},'
                                         '{"name":"host","type":"BIGINT"}]}')
(dest / "series_meta.json").write_text('{"sim":"strims-sim","bin_ms":100}')
(dest / "job.json").write_text(json.dumps({"k": "tb-0123456789abcdef", "sim": "testbed",
                                           "test": "calibration_star",
                                           "env": {"STRIMS_SIM_CAL_VIEWERS": "2", **knobs},
                                           "row": {"trace": "tb-0123", "lag_p99_ms": 250}}))
"""

TESTBED_MATRIX = """\
kind = "testbed"
repeats = 2
host_cores = 4
cell = "star-3-10"
knobs = {{ STRIMS_SIM_ARRIVALS = "trickle" }}
[repeat_env]
JOINS = ["1:3 2:9 3:14", "1:5 2:6 3:18", "1:1 2:2 3:3"]
[artifacts]
NODE_BIN = "node"
TABLES_BIN = "tables"
[arms]
main = "{commit}"
[env]
SECS = "30"
"""


class TestbedTables(unittest.TestCase):
    """A testbed run's tables, from the worker's fold to the controller's
    line (the module docs)."""

    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-farm-testbed-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        if str(FARM_PY.parent) not in sys.path:
            sys.path.insert(0, str(FARM_PY.parent))
        spec = importlib.util.spec_from_file_location("sim_farm_worker_tb", FARM_PY.parent / "worker.py")
        self.worker = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.worker)

    def host_root(self):
        """A host's farm dir whose bundle `b.tar.gz` holds the stand-in."""
        root = self.dir / "farm"
        self.worker.dirs(root)
        arts = root / "trees" / "b" / "artifacts"
        arts.mkdir(parents=True)
        (arts / "TABLES_BIN").write_text(FAKE_TABLES)
        (arts / "TABLES_BIN").chmod(0o755)
        return root

    def test_a_runs_tables_fold_as_its_cells_run_and_flush(self):
        root = self.host_root()
        out = root / "work" / "j1" / "out"
        out.mkdir(parents=True)
        job = {"id": "j1", "kind": "testbed", "bin": "b.tar.gz", "matrix": "tb", "arm": "main", "repeat": 2,
               "cell": "star-3-10", "knobs": {"STRIMS_SIM_ARRIVALS": "trickle"}, "fields": [],
               "commit": "c" * 40, "binary_sha256": "b" * 64}
        dest, meta, why = self.worker.testbed_tables(root, job, out)
        self.assertIsNone(why)
        self.assertEqual(dest, out / "tables")
        self.assertEqual(meta["env"]["STRIMS_SIM_ARRIVALS"], "trickle")
        sink = self.dir / "sink"
        (root / "policy.json").write_text(json.dumps({"dir": str(sink)}))
        args = mock.Mock(run="r1", instance="i-1", policy=str(root / "policy.json"), rotate_mb=100.0, duckdb=None)
        with mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.dir / "sim-db")}):
            store = self.worker.Store(root, args)
            row = {"steal_pct": 0.5, "node_lines_min": 40, "wall_s": 60.0}
            full = self.worker.lake_row(meta["row"], row)
            self.assertEqual({k: full[k] for k in ("_steal_share", "_node_lines_min")}, {"_steal_share": 0.005,
                                                                                      "_node_lines_min": 40})
            self.assertNotIn("_wall_s", full)
            tables = {k: meta[k] for k in ("k", "sim", "test", "env")}
            store.add("j1", {**job, "tables": tables}, {"id": "j1", "row": row, "key": meta["k"]}, full, dest)
            store.close(60)
        done = json.loads((root / "done" / "j1.json").read_text())
        self.assertEqual((done["flush"], done["key"]), (0, "tb-0123456789abcdef"))
        flushed = sink / "r1" / "i-1" / "0"
        runs = self.worker.store().duck(
            f"SELECT k, sim, test, cell, arm, seed, env, row_json, commit, binary_sha256 "
            f"FROM read_parquet('{flushed / 'runs.parquet'}');")
        self.assertEqual([(r["k"], r["sim"], r["test"], r["cell"], r["arm"], r["seed"]) for r in runs],
                         [("tb-0123456789abcdef", "testbed", "calibration_star", "star-3-10", "main", 2)])
        # Its provenance, as a test job's: the arm's commit, the bundle's sha.
        self.assertEqual((runs[0]["commit"], runs[0]["binary_sha256"]), ("c" * 40, "b" * 64))
        self.assertEqual(json.loads(runs[0]["env"])["STRIMS_SIM_ARRIVALS"], "trickle")
        self.assertEqual(json.loads(runs[0]["row_json"]), {**full})
        self.assertTrue((flushed / "viewers.parquet").exists())

    def test_a_stored_runs_tables_are_the_folders_and_their_failure_fails_it(self):
        root = self.host_root()
        sink = self.dir / "sink"
        (root / "policy.json").write_text(json.dumps({"dir": str(sink)}))
        args = mock.Mock(run="r1", instance="i-1", policy=str(root / "policy.json"), rotate_mb=100.0, duckdb=None)
        job = {"id": "j1", "kind": "testbed", "bin": "b.tar.gz", "matrix": "tb", "arm": "main", "repeat": 1,
               "cell": "star-3-10", "knobs": {}, "fields": [], "tables_pending": True}
        host = {"steal_pct": 0.5, "node_lines_min": 40, "wall_s": 60.0}
        outs = {}
        for jid in ("j1", "j2"):
            out = root / "work" / jid / "out"
            out.mkdir(parents=True)
            outs[jid] = out
        (outs["j2"] / "fail").touch()
        with mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.dir / "sim-db")}):
            store = self.worker.Store(root, args)
            for jid, out in outs.items():
                store.add(jid, {**job, "id": jid}, {"id": jid, "row": dict(host)}, dict(host), out)
            store.close(60)
        done = json.loads((root / "done" / "j1.json").read_text())
        self.assertEqual((done["flush"], done["key"]), (0, "tb-0123456789abcdef"))
        self.assertEqual(done["row"]["steal_pct"], 0.5)
        failed = json.loads((root / "done" / "j2.json").read_text())
        self.assertIsNone(failed["row"])
        self.assertIn("no node lines", failed["error"])
        self.assertEqual(failed["tables_error"], failed["error"])
        self.assertNotIn("flush", failed)

    def test_tables_that_fail_say_why(self):
        root = self.host_root()
        out = root / "work" / "j2" / "out"
        out.mkdir(parents=True)
        (out / "fail").touch()
        dest, meta, why = self.worker.testbed_tables(root, {"bin": "b.tar.gz"}, out)
        self.assertEqual((dest, meta), (None, None))
        self.assertIn("no node lines", why)
        # A bundle without TABLES_BIN writes none, and says nothing.
        (root / "trees" / "b" / "artifacts" / "TABLES_BIN").unlink()
        self.assertEqual(self.worker.testbed_tables(root, {"bin": "b.tar.gz"}, out), (None, None, None))

    def test_a_run_with_a_node_that_never_ran_fails(self):
        root = self.host_root()
        work = root / "work" / "j3"
        out = work / "out"
        out.mkdir(parents=True)
        (out / "node-0.jsonl").write_text('{"t_ms": 1}\n')
        (out / "node-1.jsonl").write_text("")
        (out / "node-1.err").write_text("testbed_node: --join-at: invalid digit found in string\n")
        r = mock.Mock(at=1.0, job={"id": "j3"})
        res = self.worker.finish_testbed(root, r, work, True)
        self.assertNotIn("row", res)
        self.assertEqual(res["error"], "node-1.jsonl without lines: testbed_node: --join-at: invalid digit found "
                                       "in string")

    def test_a_stored_runs_line_waits_for_its_flush(self):
        d = self.dir
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        env |= {"GIT_DIR": str(d / ".git"), "GIT_WORK_TREE": str(d), "GIT_CONFIG_NOSYSTEM": "1",
                "HOME": str(d), "SIM_DB_ROOT": str(d / "sim-db")}
        p = mock.patch.dict(os.environ, env, clear=True)
        p.start()
        self.addCleanup(p.stop)
        subprocess.run(["git", "init", "-q"], cwd=d, check=True)
        (d / "crates").mkdir()
        (d / "crates" / "x").write_text("x")
        for cmd in (["add", "crates"], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "x"]):
            subprocess.run(["git", *cmd], cwd=d, check=True)
        commit = subprocess.run(["git", "rev-parse", "HEAD"], cwd=d, check=True, capture_output=True,
                                text=True).stdout.strip()
        fp = farm_of(d)
        (d / "tb.toml").write_text(TESTBED_MATRIX.format(commit=commit))
        m = fp.Matrix(str(d / "tb.toml"))
        m.resolve()
        self.assertEqual((m.cell, m.knobs), ("star-3-10", {"STRIMS_SIM_ARRIVALS": "trickle"}))
        # Each repeat takes its own JOINS, which its id hashes.
        jobs = sorted(m.jobs(), key=lambda j: j.repeat)
        self.assertEqual([dict(j.env)["JOINS"] for j in jobs], ["1:3 2:9 3:14", "1:5 2:6 3:18"])
        self.assertEqual(len({j.id for j in jobs}), 2)
        short = d / "short.toml"
        short.write_text(TESTBED_MATRIX.format(commit=commit).replace("repeats = 2", "repeats = 4"))
        with self.assertRaises(SystemExit) as e:
            fp.Matrix(str(short))
        self.assertIn("fewer values than the 4 repeats", str(e.exception))
        args = mock.Mock(local=True, s3_region=None, s3_bucket=None, verify=0)
        with contextlib.redirect_stdout(io.StringIO()):
            run = fp.Run([m], args)
            m.state.mkdir(parents=True, exist_ok=True)
        self.assertTrue(run.stores)
        host = fp.Host("local", "local", "-", fp.Spec(1, 1, 0, 1), 0.0, time.time(), state="ready")
        run.sink_dir = d / "sink"
        first, second = sorted(run.queue, key=lambda i: i.job.repeat)
        w = run.wire(first, host)
        self.assertEqual((w["cell"], w["knobs"], w["repeat"], w["matrix"], w["arm"], w["commit"]),
                         ("star-3-10", {"STRIMS_SIM_ARRIVALS": "trickle"}, 1, "tb", "main", first.job.commit))
        self.assertIn("binary_sha256", w)
        rows = m.state / "rows.jsonl"
        res = {"row": {"steal_pct": 0.5}, "flush": 3, "key": "tb-0123456789abcdef", "wall_s": 60.0, "exit": 0}
        run.store(host, first, res)
        run.flush_rows()
        self.assertFalse(rows.exists())
        self.assertEqual(host.stored, {first.wid: 3})
        run.land(host, 3, {"reads": False})
        run.flush_rows()
        line, = fp.read_jsonl(rows)
        self.assertEqual((line["id"], line["key"], line["src"], line["repeat"]),
                         (first.job.id, "tb-0123456789abcdef", "farm/" + run.name + "/local/3", 1))
        self.assertEqual(line["row"], {"steal_pct": 0.5})
        self.assertEqual(host.acks, [first.wid])
        # One that stored nothing (no TABLES_BIN) has its line at once.
        run.store(host, second, {"row": {"steal_pct": 0.1}, "wall_s": 60.0, "exit": 0})
        self.assertEqual([r.get("key") for r in fp.read_jsonl(rows)], ["tb-0123456789abcdef", None])

    def test_a_testbed_runs_estimate_prices_a_host_per_group(self):
        d = self.dir
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        env |= {"GIT_DIR": str(d / ".git"), "GIT_WORK_TREE": str(d), "GIT_CONFIG_NOSYSTEM": "1",
                "HOME": str(d), "SIM_DB_ROOT": str(d / "sim-db")}
        p = mock.patch.dict(os.environ, env, clear=True)
        p.start()
        self.addCleanup(p.stop)
        subprocess.run(["git", "init", "-q"], cwd=d, check=True)
        (d / "hack" / "testbed").mkdir(parents=True)
        commits = []
        for arm in ("main", "other"):
            (d / "hack" / "testbed" / "run.sh").write_text(arm)
            for cmd in (["add", "hack"], ["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", arm]):
                subprocess.run(["git", *cmd], cwd=d, check=True)
            commits.append(subprocess.run(["git", "rev-parse", "HEAD"], cwd=d, check=True, capture_output=True,
                                          text=True).stdout.strip())
        commit, other = commits
        fp = farm_of(d)
        # Two arms paired, 3 repeats, two sizes: 6 groups of 2 jobs.
        (d / "e.toml").write_text(TESTBED_MATRIX.format(commit=commit).replace("repeats = 2", "repeats = 3")
                                  .replace('[arms]\nmain = "', '[arms]\nother = "' + other + '"\nmain = "')
                                  + '[axes]\nVIEWERS = ["3", "15"]\n')
        m = fp.Matrix(str(d / "e.toml"))
        m.resolve()
        m.state.mkdir(parents=True)
        # The 3-viewer jobs of arm main have an earlier run of 100 s.
        fp.append_jsonl(m.state / "rows.jsonl", [{"id": "old", "kind": "testbed", "arm": "main", "wall_s": 100.0,
                                                  "env": {"SECS": "30", "VIEWERS": "3", "JOINS": "x"}}])
        args = mock.Mock(local=False, s3_region="r1", s3_bucket="b", verify=0, max_vcpus=None, work_per_boot=2)
        with contextlib.redirect_stdout(io.StringIO()):
            run = fp.Run([m], args)
        cheap = fp.Pool("c6a.xlarge", "r1a", "s", fp.Spec(4, 4, 8, 1), 0.04, "r1")
        dear = fp.Pool("c7a.xlarge", "r1b", "s", fp.Spec(4, 4, 8, 1), 0.05, "r1")
        run.pools = [cheap, dear]
        run.regions = {"r1": fp.Region("r1", "", {}, "", ("", ""), budget=16)}
        est = run.estimate()
        self.assertEqual((est["jobs"], est["timed"]), (12, 6))
        # The budget's 16 vCPUs hold 4 hosts: the cheap type's pool takes
        # its half of the budget, the next pool the rest.
        self.assertEqual((est["instances"], est["slots"], est["slots_by_region"]), (4, 4, {"r1": 4}))
        model = {v: fp.testbed_model_s({"VIEWERS": v, "SECS": "30"}, True) for v in ("3", "15")}
        self.assertEqual(model["3"], 8 + 30 + 15 + fp.TABLES_S)
        self.assertEqual(model["15"], 8 + 1.5 * 12 + 30 + 15 + fp.TABLES_S)
        # Each 3-viewer group: both arms timed by the earlier 100 s (arms
        # aside), each 15-viewer group by the model.
        loads = sorted([200.0] * 3 + [2 * model["15"]] * 3, reverse=True)
        busy = [0.0] * 4
        for g in loads:
            h = busy.index(min(busy))
            busy[h] += g
        b = est["boot_s"]
        images = fp.IMAGES_S["native"]
        self.assertGreaterEqual(est["model_wall_s"], round(b + max(busy) + 2 * fp.TICK))
        self.assertLessEqual(est["model_wall_s"], round(b + max(busy) + 2 * fp.TICK + 2 * images))
        text = run.say_estimate(est)
        self.assertIn("6 timed by their matrices' earlier runs, 6 by run.sh's timing", text)
        self.assertIn("4 instances of 4 cores", text)
        self.assertEqual(set(est), {"jobs", "have_rows", "timed", "work_slot_h", "boot_s", "instances", "slots",
                                    "slots_by_region", "model_wall_s", "model_cost", "past_runs", "wall_s",
                                    "cost", "s3_cost"})
        # Two host_cores in one set: refused, as run refuses it.
        (d / "f.toml").write_text(TESTBED_MATRIX.format(commit=commit).replace("host_cores = 4", "host_cores = 8"))
        f = fp.Matrix(str(d / "f.toml"))
        f.resolve()
        with self.assertRaises(SystemExit) as e, contextlib.redirect_stdout(io.StringIO()):
            fp.Run([m, f], args)
        self.assertIn("want one host_cores", str(e.exception))


if __name__ == "__main__":
    unittest.main()
