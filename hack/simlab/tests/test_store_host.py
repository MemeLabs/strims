#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The farm hosts' store as a host gets it: farm.py's HOST_FILES copied
into an empty directory, as its instances' farm/, and the worker imported
there by a Python with neither the repo nor any site-packages on its path
(`-I -S`, so no duckdb module), whose store (store_host.py, beside it)
folds and exports both sims' jobs (golden_cli's) through the pinned DuckDB
CLI into the manifest and files simdb.py's fold and export give; every
module the host loads is the standard library's or a file it was sent:

  python3 hack/simlab/tests/test_store_host.py
"""

import json
import os
import shutil
import subprocess
import sys
import sysconfig
import tempfile
import unittest
from pathlib import Path
from unittest import mock

HACK = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(HACK))

from simlab import bootstrap  # noqa: E402

if __name__ == "__main__":
    bootstrap.engine(__file__)

import golden_cli  # noqa: E402
from simlab import engine, store_host  # noqa: E402

# On the host: the worker, from the files it was sent, folds and exports
# a flush's jobs, and says what it loaded.
HOST = """
import json, sys
farm, jobs, db, dest = sys.argv[1:]
sys.path.insert(0, farm)
import worker
store = worker.store()
wrote = store.fold(db, json.loads(open(jobs).read()), "farm/sr0001-00+0more-aaaa/i-1/0", threads=2, memory="1GB")
manifest = store.export(db, dest, threads=2)
files = sorted({m.__file__ for m in list(sys.modules.values()) if getattr(m, "__file__", None)})
print(json.dumps({"wrote": wrote, "manifest": manifest, "store": store.__file__, "pins": store.DUCKDB_VERSION,
                  "files": files, "duckdb": "duckdb" in sys.modules}))
"""


def host_files():
    """farm.py's HOST_FILES, as an importer of farm.py reads them."""
    sys.path.insert(0, str(HACK / "sim-farm"))
    try:
        import farm
    finally:
        sys.path.remove(str(HACK / "sim-farm"))
    return farm.HOST_FILES


class Host(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory(prefix="simlab-host-")
        self.addCleanup(tmp.cleanup)
        self.dir = Path(tmp.name)
        p = mock.patch.dict(os.environ, {"SIM_DB_ROOT": str(self.dir / "here")})
        p.start()
        self.addCleanup(p.stop)
        (self.dir / "here").mkdir()

    def test_a_host_folds_and_exports_with_the_files_it_is_sent(self):
        farm = self.dir / "farm"
        farm.mkdir()
        sent = host_files()
        for f in sent:
            shutil.copy2(f, farm / Path(f).name)
        self.assertIn("store_host.py", {Path(f).name for f in sent})
        jobs = [golden_cli.job_dir(self.dir / "jobs" / f"{i:02d}", j) for i, j in enumerate(golden_cli.JOBS[:4])]
        self.assertEqual({j["sim"] for j in jobs}, {"ppspp", "strims-sim"})
        (self.dir / "jobs.json").write_text(json.dumps(jobs))
        cli = store_host.duckdb_bin()
        env = {"PATH": os.environ.get("PATH", ""), "HOME": str(self.dir), "SIM_DB_ROOT": str(self.dir / "store"),
               "SIM_DUCKDB": str(cli)}
        (self.dir / "store").mkdir()
        p = subprocess.run([sys.executable, "-I", "-S", "-c", HOST, str(farm), str(self.dir / "jobs.json"),
                            str(self.dir / "store" / "gen-0.duckdb"), str(self.dir / "flush")],
                           cwd=self.dir / "store", env=env, capture_output=True, text=True)
        self.assertEqual(p.returncode, 0, p.stderr)
        got = json.loads(p.stdout)
        self.assertEqual(Path(got["store"]), farm / "store_host.py")
        self.assertEqual(got["pins"], engine.DUCKDB_VERSION)
        self.assertFalse(got["duckdb"])
        stdlib = [Path(sysconfig.get_paths()[k]).resolve() for k in ("stdlib", "platstdlib")]
        for f in map(lambda x: Path(x).resolve(), got["files"]):
            self.assertTrue(f.parent == farm.resolve() or any(f.is_relative_to(d) for d in stdlib), f)
        # What simdb.py's fold and export (the same store, in the CLI)
        # give of the same jobs.
        sm = golden_cli.simdb_test().simdb
        with mock.patch.object(store_host, "EXECUTOR", None):
            wrote = sm.fold(self.dir / "here.duckdb", jobs, "farm/sr0001-00+0more-aaaa/i-1/0", threads=2)
            want = sm.export(self.dir / "here.duckdb", self.dir / "want", threads=2)
        self.assertEqual(got["wrote"], wrote)
        self.assertEqual(dict(got["manifest"], at=None), dict(want, at=None))
        self.assertEqual(sorted(p.name for p in (self.dir / "flush").iterdir()),
                         sorted(p.name for p in (self.dir / "want").iterdir()))


if __name__ == "__main__":
    unittest.main()
