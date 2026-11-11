#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The farm matrices and index `hack/sim-suite/suite.py farm --no-run`
writes, byte for byte, for every tier, against
testdata/golden/matrices.txt.gz:

  python3 hack/simlab/tests/golden_suite.py check
  python3 hack/simlab/tests/golden_suite.py capture

Each run is `suite.py farm --no-run` through suite.py's own main(), loaded
afresh, with --sets (the arms' commit and the knob's values) and with
--refs (commits as the arms) for each tier, then a few runs of the other
options (--seeds, --first-seed, --env, --question, --against, --no-off,
--knob, --only) on one tier. The commits are two of main's, so the
matrices' arms and the cells' memory (peak_gib, over no kept rows) stay
put. It writes nowhere shared: simlab.suite's seams, whose
write_matrices suite.py's farm calls, are pointed at a scratch dir for
its run (WORK, its rows and matrices; farm_root, the main checkout whose
target/ holds the farm's and the suite's state), and the scoring after
the matrices is left out (simlab.suite's farm_rows, the rows the farm
holds, gives none; simlab.scoring's arm scores and scorecards print
nothing), so nothing pulls, indexes or scores. A run's text is its
output, then each file it wrote (the matrices in target/sim-suite/farm/,
the index in farm-runs/ or pending/), named relative to the scratch dir.
"""

import argparse
import collections
import contextlib
import importlib.util
import io
import sys
import tempfile
from pathlib import Path
from unittest import mock

import goldens

HACK = Path(__file__).resolve().parents[2]
SUITE = HACK / "sim-suite" / "suite.py"
GOLDEN = goldens.TESTDATA / "matrices.txt.gz"
A, B = "a9f05b4b62895bd6cb24829871797bd700fca6ce", "33d0d008a64b272005a892282735af17784349c8"


def suite():
    spec = importlib.util.spec_from_file_location("suite", SUITE)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def runs():
    """(name, argv) of every run."""
    out = []
    for tier in sorted(suite().TIERS):
        out.append((f"{tier}-sets", ["--tier", tier, "--ref", A, "--sets", "x;y,z"]))
        out.append((f"{tier}-refs", ["--tier", tier, "--refs", f"base={A},new={B}"]))
    out += [
        ("peers-options", ["--tier", "peers", "--ref", A, "--sets", "x", "--seeds", "4", "--first-seed", "3001",
                           "--env", "SIM_DEPART=60:40:10", "--env", "STRIMS_SIM_JOB=2", "--question", "does x help?",
                           "--no-off", "--only", "churn,star-31-10"]),
        ("robust-against", ["--tier", "robust", "--ref", A, "--sets", "x;y", "--against", "x",
                            "--knob", "SIM_KNOB,STRIMS_SIM_KNOB", "--only", "star-7-5,halve-15-10"]),
        ("smoke-bad-against", ["--tier", "smoke", "--ref", A, "--sets", "x", "--against", "nope"]),
    ]
    return out


def farm(argv):
    """What `suite.py farm --no-run ARGV` prints and writes."""
    s = suite()
    with tempfile.TemporaryDirectory(prefix="simlab-matrices-") as tmp, contextlib.ExitStack() as seams:
        tmp = Path(tmp)
        for name, value in (("WORK", tmp / "work"), ("farm_root", lambda: tmp / "main"),
                            ("farm_rows", lambda name, cells, knob: collections.defaultdict(dict))):
            seams.enter_context(mock.patch.object(s.lab, name, value))
        for name in ("arm_score", "scorecard"):
            seams.enter_context(mock.patch.object(s.scoring, name, lambda *a, **kw: mock.Mock(text=lambda: "")))
        out, err = io.StringIO(), io.StringIO()
        old = sys.argv
        sys.argv = ["suite.py", "farm", "--no-run", *argv]
        code = 0
        try:
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                s.main()
        except SystemExit as e:
            code = e.code if isinstance(e.code, int) else (print(e.code, file=err) or 1)
        finally:
            sys.argv = old
        text = out.getvalue() + (f"--- stderr\n{err.getvalue()}" if err.getvalue() else "") + f"--- exit {code}\n"
        for f in sorted(p for p in tmp.rglob("*") if p.is_file()):
            text += f"=== {f.relative_to(tmp)}\n{f.read_text()}"
        return text


def outputs():
    return {name: farm(argv) for name, argv in runs()}


def check():
    return goldens.compare("golden_suite", goldens.read(GOLDEN), outputs())


def capture():
    goldens.write(GOLDEN, outputs())
    print(f"wrote {GOLDEN}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=("check", "capture"))
    a = ap.parse_args()
    if a.cmd == "capture":
        capture()
        return
    sys.exit(0 if check() else 1)


if __name__ == "__main__":
    main()
