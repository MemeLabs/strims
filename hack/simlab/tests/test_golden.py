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
               lines and its log
  Passing      a set that passes prints its last line alone and takes its
               set's log from an earlier failure away

  python3 hack/simlab/tests/test_golden.py
"""

import contextlib
import io
import sys
import tempfile
import textwrap
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
