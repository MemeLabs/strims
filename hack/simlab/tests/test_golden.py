#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""golden.py's report of a set run as a script (golden.script), over
fixture sets, stdlib alone:

  Failing      a unittest set whose failures and error print pages each,
               the first of them far ahead of its output's last 4000
               characters: every one named, the whole output (stderr
               where it came) in its log, whose path is printed, and
               little else printed
  Scorecards   a set printing as hack/sim-suite/golden.py does, diffs and
               a line an experiment, several scored otherwise: each of
               those named, none of those scored the same
  Crash        a set that names no failure (a traceback): its last TAIL
               lines and its log; one that crashes after the failures it
               names: those, then the crash's traceback
  Groups       the groups, each under 15 minutes, hold every set once,
               in order; simlab's tests split between the engine and the
               catalog sets
  Unbuffered   a check whose stdout is a pipe, killed part-way, has
               printed the line of each set it finished
  Passing      a set that passes prints its last line alone and takes its
               set's log from an earlier failure away

  python3 hack/simlab/tests/test_golden.py
"""

import contextlib
import io
import os
import subprocess
import sys
import tempfile
import textwrap
import threading
import unittest
from pathlib import Path
from unittest import mock

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import golden  # noqa: E402

PAGE = "x" * 100 + "\n"


class Fixture(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-golden-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        p = mock.patch.object(golden, "LOGS", self.dir / "logs")
        p.start()
        self.addCleanup(p.stop)

    def run_set(self, source, label="fixture"):
        """golden.script over a set that is the script `source`: whether it
        passed, and what it printed."""
        path = self.dir / f"{label}.py"
        path.write_text(textwrap.dedent(source))
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            ok = golden.script([sys.executable, str(path)], label)
        return ok, out.getvalue()

    def log(self, label="fixture"):
        return (self.dir / "logs" / f"{label}.log").read_text()


class Failing(Fixture):
    SET = f"""
        import sys
        import unittest

        PAGE = {PAGE!r}

        class Cases(unittest.TestCase):
            def test_a_first(self):
                print("first's own output", flush=True)
                self.fail("first fails\\n" + PAGE * 40)

            def test_b_passes(self):
                pass

            def test_c_second(self):
                self.assertEqual("second", "other " + PAGE * 40)

            def test_d_errs(self):
                raise KeyError("the error")

            def test_e_last(self):
                sys.stderr.write(PAGE * 40)
                self.fail("last fails\\n" + PAGE * 40)

        unittest.main()
    """

    def test_every_failure_named_and_the_whole_output_in_its_log(self):
        ok, out = self.run_set(self.SET)
        self.assertFalse(ok)
        log = self.log()
        # The output's last 4000 characters, all a failing set printed
        # before, name none of them.
        self.assertEqual(golden.failures(log[-4000:].splitlines()), [])
        lines = out.splitlines()
        self.assertTrue(lines[0].startswith("fixture: FAILED ("), lines[0])
        self.assertTrue(lines[0].endswith(": FAILED (failures=3, errors=1)"), lines[0])
        named = [line.strip().split(" (")[0] for line in lines[1:-1]]
        # As unittest reports them: errors, then failures.
        self.assertEqual(named, ["ERROR: test_d_errs", "FAIL: test_a_first", "FAIL: test_c_second",
                                 "FAIL: test_e_last"])
        self.assertEqual(lines[-1], f"  fixture: 4 failures named; all {len(log.splitlines())} lines in "
                                    f"{self.dir / 'logs' / 'fixture.log'}")
        # The log holds it all, the test's stdout where it came.
        for text in ("first's own output", "first fails", "AssertionError: 'second' != 'other ", "KeyError",
                     "last fails", "Ran 5 tests"):
            self.assertIn(text, log)
        self.assertLess(log.index("first's own output"), log.index("FAIL: test_a_first"))


class Scorecards(Fixture):
    SET = f"""
        import sys

        PAGE = {PAGE!r}
        for exp, same in (("s07cfc", False), ("s74ae3", False), ("s704e2", True), ("s2e215", False),
                          ("sa8a4b", False), ("s010c3", True)):
            for how in ("suite.py", "arm_side"):
                if not same:
                    print("--- reference\\n+++ " + how + " " + exp + "\\n" + "-" + PAGE * 30 + "+" + PAGE * 30, end="")
                print(f"{{exp}} trains-seed against off ({{how}}): {{'same' if same else 'DIFFERENT'}}; 30 cells, "
                      "12 marked lines, moved: 1 better, 0 worse; guardrails none")
        sys.exit(1)
    """

    def test_each_experiment_scored_otherwise_named(self):
        ok, out = self.run_set(self.SET, "scorecards")
        self.assertFalse(ok)
        named = [line.strip().split(":")[0] for line in out.splitlines()[1:-1]]
        self.assertEqual(named, [f"{exp} trains-seed against off ({how})" for exp in ("s07cfc", "s74ae3", "s2e215",
                                                                                      "sa8a4b")
                                 for how in ("suite.py", "arm_side")])
        self.assertIn("scorecards: 8 failures named; all ", out.splitlines()[-1])
        self.assertIn("s704e2 trains-seed against off (arm_side): same", self.log("scorecards"))


class Crash(Fixture):
    def test_a_set_naming_no_failure_prints_its_tail(self):
        ok, out = self.run_set(f"""
            print({PAGE!r} * 100, end="")
            raise RuntimeError("the set broke")
        """)
        self.assertFalse(ok)
        lines = out.splitlines()
        self.assertEqual(lines[0].split(": ", 2)[2], "RuntimeError: the set broke")
        self.assertEqual(lines[-2], "  RuntimeError: the set broke")
        self.assertEqual(len(lines), 1 + golden.TAIL + 1)
        self.assertIn(f"fixture: no failure named: its last {golden.TAIL} lines; all ", lines[-1])
        self.assertIn("Traceback", self.log())

    def test_a_crash_after_named_failures_prints_its_traceback_too(self):
        ok, out = self.run_set(f"""
            import unittest

            class Cases(unittest.TestCase):
                def test_fails(self):
                    self.fail("it fails\\n" + {PAGE!r} * 40)

            unittest.main(exit=False)
            print({PAGE!r} * 100, end="")
            raise RuntimeError("the set broke after its tests")
        """)
        self.assertFalse(ok)
        lines = out.splitlines()
        self.assertTrue(lines[1].startswith("  FAIL: test_fails"), lines[1])
        self.assertEqual(lines[2], "  Traceback (most recent call last):")
        self.assertEqual(lines[-2], "  RuntimeError: the set broke after its tests")
        self.assertNotIn(PAGE.strip(), out)
        self.assertIn("fixture: 1 failures named, then a crash's traceback (", lines[-1])
        # A failing test's traceback, in unittest's report, is the failure's.
        self.assertEqual(golden.crashed(["FAIL: x", "Traceback (most recent call last):", "  ...", "Ran 1 test in 0.1s",
                                         "FAILED (failures=1)"]), [])


class Groups(unittest.TestCase):
    def test_the_groups_hold_every_set_once_in_order(self):
        self.assertEqual([s for g in golden.GROUPS for s in g], list(golden.SETS))
        # simlab's tests split between the engine set and the catalog's.
        every = sorted(p.stem for p in HERE.glob("test_*.py"))
        engine, catalog = golden.unit_tests("engine"), golden.unit_tests("catalog")
        self.assertEqual(catalog, ["test_catalog"])
        self.assertEqual(sorted(engine + catalog), every)
        self.assertIn("test_golden", engine)


class Unbuffered(Fixture):
    def test_a_killed_check_has_printed_the_sets_it_finished(self):
        # golden.py check of two sets, its stdout a pipe, killed during the
        # second: the first's line is out.
        script = self.dir / "check.py"
        script.write_text(textwrap.dedent(f"""
            import sys, time
            sys.path.insert(0, {str(HERE)!r})
            import golden

            def run_set(name):
                print(f"{{name}}: ok (0 s): done")
                if name == "matrices":
                    time.sleep(120)
                return True

            golden.run_set = run_set
            sys.argv = ["golden.py", "check", "--only", "cli,matrices"]
            golden.main()
        """))
        p = subprocess.Popen([sys.executable, str(script)], stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True,
                             env={k: v for k, v in os.environ.items() if k != "PYTHONUNBUFFERED"})
        self.addCleanup(p.stdout.close)
        self.addCleanup(p.wait)
        self.addCleanup(p.kill)
        got = []
        reader = threading.Thread(target=lambda: got.append(p.stdout.readline()), daemon=True)
        reader.start()
        reader.join(60)
        p.kill()
        self.assertEqual(got, ["cli: ok (0 s): done\n"])


class Passing(Fixture):
    def test_a_passing_set_prints_its_last_line_and_drops_an_old_log(self):
        self.assertFalse(self.run_set("raise SystemExit(1)")[0])
        self.assertTrue((self.dir / "logs" / "fixture.log").exists())
        ok, out = self.run_set("print('a page'); print('all good')")
        self.assertTrue(ok)
        self.assertRegex(out, r"^fixture: ok \(\d+ s\): all good\n$")
        self.assertFalse((self.dir / "logs" / "fixture.log").exists())


if __name__ == "__main__":
    unittest.main()
