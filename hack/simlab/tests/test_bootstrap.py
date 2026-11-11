#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""simlab.bootstrap: a script run by a Python without duckdb runs again
under hack/'s uv venv, from any directory, its arguments, stdin and cwd
kept; one that has it runs as it is; a module that loads the script by
path never runs it; without uv, or still without duckdb under it, it says
why and exits. Its scripts run under the system python3 (stdlib alone):

  python3 hack/simlab/tests/test_bootstrap.py
"""

import json
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

# A wrapper as each script's __main__ block has it, printing where it ran.
SCRIPT = f"""\
import json, os, sys
from pathlib import Path
sys.path.insert(0, {str(HACK)!r})
LOADED = True
if __name__ == "__main__":
    from simlab import bootstrap
    bootstrap.engine(__file__)
    import duckdb
    print(json.dumps({{"exe": sys.executable, "duckdb": duckdb.__version__, "argv": sys.argv[1:],
                      "cwd": os.getcwd(), "uv": os.environ.get("SIMLAB_UV"), "stdin": sys.stdin.read()}}))
"""
# A Python without the duckdb module (no site packages): the system one.
PLAIN = "/usr/bin/python3"


def plain_python():
    p = subprocess.run([PLAIN, "-I", "-c", "import duckdb"], capture_output=True)
    return p.returncode != 0


@unittest.skipUnless(Path(PLAIN).exists() and plain_python(), "needs a python3 without the duckdb module")
class Bootstrap(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-bootstrap-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        self.script = self.dir / "wrapper.py"
        self.script.write_text(SCRIPT)
        self.env = {k: v for k, v in os.environ.items() if k != bootstrap.ENV}

    def run_it(self, *args, python=PLAIN, cwd=None, env=None, stdin=""):
        return subprocess.run([python, *args], capture_output=True, text=True, cwd=cwd or self.dir,
                              env=env or self.env, input=stdin, timeout=300)

    def test_a_python_without_duckdb_runs_the_script_under_uv(self):
        p = self.run_it("-I", str(self.script), "sql", "SELECT 'a b'", "--json", stdin="piped", cwd="/")
        self.assertEqual(p.returncode, 0, p.stderr)
        got = json.loads(p.stdout)
        self.assertEqual(Path(got["exe"]).parent, HACK / ".venv" / "bin")
        self.assertEqual((got["duckdb"], got["argv"], got["cwd"], got["uv"], got["stdin"]),
                         ("1.5.6", ["sql", "SELECT 'a b'", "--json"], "/", "1", "piped"))
        self.assertEqual(p.stderr, "")

    def test_uv_run_of_the_script_works_too(self):
        p = subprocess.run([bootstrap.uv(), "run", "--no-project", "--quiet", str(self.script), "x"],
                           capture_output=True, text=True, cwd="/", env=self.env, timeout=300)
        self.assertEqual(p.returncode, 0, p.stderr)
        self.assertEqual(json.loads(p.stdout)["argv"], ["x"])

    def test_a_python_with_duckdb_runs_it_as_it_is(self):
        venv = HACK / ".venv" / "bin" / "python"
        p = self.run_it(str(self.script), "y", python=str(venv))
        self.assertEqual(p.returncode, 0, p.stderr)
        got = json.loads(p.stdout)
        self.assertEqual((got["exe"], got["uv"]), (str(venv), None))

    def test_a_load_by_path_never_runs_it(self):
        code = (f"import importlib.util, os\n"
                f"os.execv = None\n"
                f"spec = importlib.util.spec_from_file_location('wrapper', {str(self.script)!r})\n"
                f"m = importlib.util.module_from_spec(spec); spec.loader.exec_module(m)\n"
                f"print(m.LOADED, m.__name__)\n")
        p = self.run_it("-I", "-c", code)
        self.assertEqual((p.returncode, p.stdout, p.stderr), (0, "True wrapper\n", ""))

    def test_under_uv_still_without_duckdb_it_exits(self):
        p = self.run_it("-I", str(self.script), env={**self.env, bootstrap.ENV: "1"})
        self.assertEqual(p.returncode, 1)
        self.assertIn("no duckdb module", p.stderr)

    def test_without_uv_it_says_to_install_it(self):
        with mock.patch.object(bootstrap, "uv", lambda: None), mock.patch.object(bootstrap, "has_engine",
                                                                                 lambda: False), \
                mock.patch.dict(os.environ, {k: v for k, v in os.environ.items() if k != bootstrap.ENV}, clear=True):
            with self.assertRaises(SystemExit) as cm:
                bootstrap.engine("x.py")
        self.assertIn("needs uv", str(cm.exception.code))
        self.assertIn(bootstrap.INSTALL, str(cm.exception.code))

    def test_uv_is_found_off_the_path(self):
        home = self.dir / "home"
        (home / ".local" / "bin").mkdir(parents=True)
        fake = home / ".local" / "bin" / "uv"
        fake.write_text("#!/bin/sh\n")
        fake.chmod(0o755)
        with mock.patch.dict(os.environ, {"PATH": str(self.dir / "empty"), "HOME": str(home)}):
            self.assertEqual(bootstrap.uv(), str(fake))
        with mock.patch.dict(os.environ, {"PATH": str(self.dir / "empty"), "HOME": str(self.dir / "nohome")}):
            self.assertIsNone(bootstrap.uv())


if __name__ == "__main__":
    unittest.main()
