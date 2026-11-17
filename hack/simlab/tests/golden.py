#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Every golden of the sims' results tools, in one command: what each
script prints must stay what it printed before simlab took its work over.

  python3 hack/simlab/tests/golden.py check [--only SET,...] [--skip SET,...]
  python3 hack/simlab/tests/golden.py capture [--only SET,...]
  python3 hack/simlab/tests/golden.py real capture|check [--exps E,...] [--out DIR]

The sets, in order (check runs them all, and exits 1 if any fails):

  cli           simdb.py's commands over a fixture cache, against
                testdata/golden/cli.txt.gz (golden_cli.py)
  matrices      suite.py farm --no-run's matrices and index for every tier,
                byte for byte, against testdata/golden/matrices.txt.gz
                (golden_suite.py)
  suite_test    hack/sim-suite/suite_test.py: the scorecards and arm scores
                of its fixtures, as the Python scoring printed them
  simdb_test    hack/sim-db/simdb_test.py
  engine        simlab's own tests (test_*.py here) but test_catalog's,
                under uv, after a build of strims-sim's testbed_tables
                (test_testbed reads it)
  catalog       test_catalog.py, the catalog's index of scratch caches,
                under uv
  scorecards1, scorecards2
                hack/sim-suite/golden.py check, half the experiments each
                (--part): the latest farm experiments' scorecards and arm
                scores, the macros against the Python scoring at its
                REFERENCE, over the catalog (`scorecards` names both)

The whole check takes about 40 minutes on the dev VM, past the 15 that a
run may block, so run it as its groups (GROUPS), one at a time, each under
`timeout 900`:

  golden.py check --only cli,matrices,suite_test,simdb_test   about 5 min
  golden.py check --only engine                                about 6 min
  golden.py check --only catalog                               12-14 min
  golden.py check --only scorecards1                           about 9 min
  golden.py check --only scorecards2                           about 6 min

A set run as a script (suite_test, simdb_test, engine, catalog, the
scorecards') prints its last line. One that fails also prints the lines
that name its failures (FAILURE: each failing test's, each experiment
scored otherwise), else its last TAIL lines, and the traceback of a
crash that no failure named holds (crashed), and writes its whole output
to target/simlab/golden/check/<set>.log, whose path it prints. golden.py's
own lines go out as each is printed, so a check killed part-way (by a
`timeout`) has printed every set it finished.

`capture` rewrites the goldens of cli and matrices from today's code, for
a change meant to move what they print (say why in its commit).

`real` runs the same kinds of invocation over real experiments in the
catalog (by default the latest two farm experiments of at most four cells
and the latest local suite invocation whose reads are in the cache), into
target/simlab/golden/ (DIR): sim data is ephemeral, so they stay local.
`real capture` before a change and `real check` after it compare them,
over the experiments the capture chose (exps.json). The invocations read
the real cache and catalog (never indexing it) with an aws CLI that
refuses, so nothing reaches S3; a farm experiment's select its cells in
its farm runs' flushes (experiments.farm_runs: a run may hold several
experiments' jobs) and need no reads (the farm keeps them in S3).
"""

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

import golden_cli
import golden_suite
import goldens

HERE = Path(__file__).resolve().parent
HACK = HERE.parents[1]
REPO = HACK.parent
SIMDB = golden_cli.SIMDB
SETS = ("cli", "matrices", "suite_test", "simdb_test", "engine", "catalog", "scorecards1", "scorecards2")
# A set's other name: the sets it stands for.
ALIASES = {"scorecards": ("scorecards1", "scorecards2")}
# The scorecards sets' parts of the experiments (hack/sim-suite/golden.py
# check --part): the check of all of them ran 895 s on the dev VM.
SCORECARD_PARTS = 2
CAPTURED = ("cli", "matrices")
# The modules of simlab's tests that the catalog set runs, apart from the
# engine set's: the longest of them.
CATALOG_TESTS = ("test_catalog",)
# The sets in groups that each run within 15 minutes (the module's docs).
GROUPS = (("cli", "matrices", "suite_test", "simdb_test"), ("engine",), ("catalog",), ("scorecards1",),
          ("scorecards2",))
sys.path.insert(0, str(HACK))
from simlab import bootstrap  # noqa: E402

UV = [bootstrap.uv() or "uv", "run", "--project", str(HACK), "--locked", "--quiet", "python"]
# Where a failing set's whole output goes, <set>.log.
LOGS = REPO / "target" / "simlab" / "golden" / "check"
# The lines of a set's output that name a failure: unittest's header of a
# test that failed or erred, and hack/sim-suite/golden.py's line of an
# experiment's arm (or a local invocation) scored otherwise than the
# reference.
FAILURE = re.compile(r"^(?:FAIL|ERROR): |: DIFFERENT; ")
# A failing set's lines printed when none names a failure (a crash), and
# the most of a crash's traceback printed beside the failures named.
TAIL = 40
# unittest's line closing its report: the failures' reports come before it.
RAN = re.compile(r"^Ran \d+ tests? in ")
TRACEBACK = "Traceback (most recent call last):"


def failures(lines):
    """The lines among `lines` that name a failure (FAILURE), in order."""
    return [line for line in lines if FAILURE.search(line)]


def crashed(lines):
    """The traceback `lines` end with that no failure they name holds: from
    the last TRACEBACK past unittest's report (its RAN line, after the
    reports of the tests that failed), or in output with no report, to the
    end, at most its last TAIL lines; else []."""
    starts = [i for i, line in enumerate(lines) if line.startswith(TRACEBACK)]
    ran = max((i for i, line in enumerate(lines) if RAN.match(line)), default=-1)
    if not starts or starts[-1] < ran:
        return []
    return lines[max(starts[-1], len(lines) - TAIL):]


def script(args, label, env=None):
    """Whether a command exits 0 (run with `env` added to this process's
    environment); its output's last line, either way. A failing one's
    whole output, stderr where it came, goes to LOGS/<label>.log, its path
    printed after the lines that name its failures, else its last TAIL
    lines."""
    t0 = time.time()
    p = subprocess.run(args, cwd=REPO, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                       text=True, env={**os.environ, "PYTHONUNBUFFERED": "1", **(env or {})})
    lines = p.stdout.rstrip().splitlines()
    print(f"{label}: {'ok' if p.returncode == 0 else 'FAILED'} ({time.time() - t0:.0f} s): {(lines or [''])[-1]}")
    log = LOGS / f"{label}.log"
    if p.returncode == 0:
        log.unlink(missing_ok=True)
        return True
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(p.stdout)
    named = failures(lines)
    crash = crashed(lines) if named else []
    print("".join(f"  {line}\n" for line in (named + crash if named else lines[-TAIL:])), end="")
    told = f"{len(named)} failures named" if named else f"no failure named: its last {min(TAIL, len(lines))} lines"
    if crash:
        told += f", then a crash's traceback ({len(crash)} lines)"
    print(f"  {label}: {told}; all {len(lines)} lines in {log}")
    return False


def unit_tests(name):
    """The modules of simlab's tests (test_*.py here) that set `name`
    runs: the catalog set CATALOG_TESTS, the engine set the others."""
    return sorted(t.stem for t in HERE.glob("test_*.py") if (t.stem in CATALOG_TESTS) == (name == "catalog"))


def unit_env():
    """What the engine and catalog sets add to the environment: this
    directory on PYTHONPATH, ahead of the caller's own."""
    return {"PYTHONPATH": os.pathsep.join([str(HERE), *filter(None, [os.environ.get("PYTHONPATH")])])}


def run_set(name):
    if name == "cli":
        return golden_cli.check()
    if name == "matrices":
        return golden_suite.check()
    if name == "suite_test":
        return script([sys.executable, str(HACK / "sim-suite" / "suite_test.py")], name)
    if name == "simdb_test":
        return script([sys.executable, str(HACK / "sim-db" / "simdb_test.py")], name)
    if name == "engine":
        # test_testbed reads tier runs' knobs through strims-sim's
        # testbed_tables, this checkout's release build: built first (an
        # incremental build), so the set tests it rather than skips it.
        if not script(["cargo", "build", "--release", "--quiet", "-p", "strims-sim", "--example", "testbed_tables"],
                      "engine-build"):
            return False
    if name in ("engine", "catalog"):
        return script([*UV, "-m", "unittest", *unit_tests(name)], name, env=unit_env())
    if name.startswith("scorecards"):
        part = f"{name.removeprefix('scorecards')}/{SCORECARD_PARTS}"
        return script([sys.executable, str(HACK / "sim-suite" / "golden.py"), "check", "--part", part], name)
    raise ValueError(name)


# --- real data ------------------------------------------------------------------------


def lake(sql):
    p = subprocess.run([sys.executable, str(SIMDB), "lake", sql, "--json"], capture_output=True, text=True,
                       check=True)
    return json.loads(p.stdout)


def choose():
    """The default experiments: the latest two farm ones of at most four
    cells, and the latest local one of 4-40 jobs whose reads are local."""
    farm = lake("SELECT exp_id FROM experiments WHERE kind = 'farm' AND NOT pending AND cells <= 4 "
                "AND len(arm_labels) >= 2 ORDER BY started_at DESC, exp_id LIMIT 2")
    local = lake("SELECT e.exp_id FROM experiments e JOIN experiment_jobs j USING (exp_id) LEFT JOIN availability a "
                 "ON a.k = j.k AND a.tbl = 'reads' WHERE e.kind = 'local' GROUP BY e.exp_id, e.started_at "
                 "HAVING count(DISTINCT j.k) = count(DISTINCT j.k) FILTER (WHERE a.state = 'local') "
                 "AND count(DISTINCT j.k) BETWEEN 4 AND 40 ORDER BY e.started_at DESC, e.exp_id LIMIT 1")
    return [r["exp_id"] for r in farm + local]


def real_invocations(exp, kind, src):
    """(name, script, args) over experiment `exp`."""
    # A farm experiment's cells, in the flushes of its farm runs (which may
    # hold other experiments' jobs too).
    src = [*src, "--cell", f"*@{exp}"] if kind == "farm" else ["--src", exp]
    q = exp.replace("'", "''")
    out = [
        ("runs", SIMDB, ["sql", "SELECT cell, arm, count(*) AS runs, min(seed) AS lo, max(seed) AS hi FROM runs "
                                "GROUP BY ALL ORDER BY ALL", *src]),
        ("verdicts", SIMDB, ["sql", "SELECT r.cell, r.arm, count(*) FILTER (WHERE v.collapsed) AS collapsed, "
                                    "round(avg(v.unhealthy), 4) AS unhealthy, round(avg(v.lag_p50), 1) AS p50, "
                                    "max(v.lag_max) AS lag_max FROM runs r JOIN verdicts v USING (k, src) "
                                    "GROUP BY ALL ORDER BY ALL", *src]),
        ("phases-json", SIMDB, ["sql", "SELECT r.cell, r.arm, vs.phase, count(*) AS secs, round(avg(vs.lag_q), 2) "
                                       "AS lag_q, sum(vs.healthy::INT) AS healthy FROM runs r JOIN verdict_seconds "
                                       "vs USING (k, src) GROUP BY ALL ORDER BY ALL", "--json", *src]),
        ("episodes", SIMDB, ["sql", "SELECT r.cell, r.arm, e.kind, count(*) AS n, sum(e.len) AS secs FROM runs r "
                                    "JOIN episodes e USING (k, src) GROUP BY ALL ORDER BY ALL", *src]),
        ("aligned-onset", SIMDB, ["aligned", "onset", "--field", "lag_q", "--after", "20", *src]),
        ("aligned-seed-down", SIMDB, ["aligned", "seed_down", "--field", "healthy", "--before", "5", *src]),
        ("events", SIMDB, ["events", *src]),
        ("osc", SIMDB, ["osc", *src]),
        ("rows", SIMDB, ["rows", "--fields", "lag_p99_ms,lag_p50_ms,unhealthy,collapsed,trace", *src]),
        ("lake-jobs", SIMDB, ["lake", f"SELECT e.cell, e.arm_label, count(*) AS jobs, count(*) FILTER (WHERE "
                                      f"v.collapsed) AS collapsed, round(avg(v.unhealthy), 4) AS unhealthy FROM "
                                      f"experiment_jobs e LEFT JOIN verdicts v USING (k, src) WHERE e.exp_id = "
                                      f"'{q}' GROUP BY ALL ORDER BY ALL"]),
        ("lake-experiment", SIMDB, ["lake", f"SELECT * EXCLUDE (finished_at) FROM experiments WHERE exp_id = '{q}'",
                                    "--json"]),
    ]
    if kind != "farm":
        out += [("pooled", SIMDB, ["pooled", *src]), ("pooled-phase", SIMDB, ["pooled", "--by", "phase", *src]),
                ("aligned-first-read", SIMDB, ["aligned", "first_read", "--per", "node", "--field", "lag_ms",
                                               "--after", "10", *src]),
                ("reads", SIMDB, ["sql", "SELECT r.cell, r.arm, count(*) AS n, sketch_quantile(lag_ms, 99) AS p99 "
                                         "FROM reads JOIN runs r USING (k, src) GROUP BY ALL ORDER BY ALL", *src])]
    return out


def real_outputs(exps, out):
    """{exp/name: text} of the real invocations, each written under `out`."""
    main = Path(subprocess.run(["git", "rev-parse", "--path-format=absolute", "--git-common-dir"], cwd=REPO,
                               capture_output=True, text=True, check=True).stdout.strip()).parent
    stand_in = out / "bin"
    stand_in.mkdir(parents=True, exist_ok=True)
    (stand_in / "aws").write_text("#!/bin/sh\necho \"aws stand-in: no S3 in the real golden ($1 $2)\" >&2\nexit 1\n")
    (stand_in / "aws").chmod(0o755)
    env = {**{k: v for k, v in os.environ.items() if k not in ("SIM_DB_ROOT", "GIT_DIR", "SIMLAB_UV")},
           "TZ": "UTC", "COLUMNS": "100", "PATH": f"{stand_in}:{os.environ.get('PATH', '')}"}
    got = {}
    for exp in exps:
        kind = "farm" if not exp.startswith("suite/") else "local"
        matrices = sorted(str(p) for p in (main / "target" / "sim-farm").glob(f"{exp}-[0-9][0-9]"))
        runs = lake(f"SELECT unnest(farm_runs) AS r FROM experiments WHERE exp_id = '{exp}'") if matrices else []
        src = [x for r in runs for x in ("--src", f"farm/{r['r']}")]
        for name, path, args in real_invocations(exp, kind, src):
            p = subprocess.run([sys.executable, str(path), *args], cwd=REPO, env=env, capture_output=True, text=True)
            err = p.stderr and "--- stderr\n" + p.stderr
            got[f"{exp}/{name}"] = p.stdout + err + f"--- exit {p.returncode}\n"
            print(f"  {exp} {name}: exit {p.returncode}", file=sys.stderr)
    return got


def real(cmd, exps, out):
    out = Path(out).resolve()
    chosen = out / "exps.json"
    golden = out / "outputs.txt.gz"
    if cmd == "capture":
        exps = exps or choose()
        out.mkdir(parents=True, exist_ok=True)
        chosen.write_text(json.dumps(exps) + "\n")
        goldens.write(golden, real_outputs(exps, out))
        print(f"real: captured {', '.join(exps)} into {out}")
        return True
    if not chosen.exists():
        sys.exit(f"no {chosen}: run golden.py real capture first (before the change)")
    exps = json.loads(chosen.read_text())
    return goldens.compare(f"real ({', '.join(exps)})", goldens.read(golden), real_outputs(exps, out))


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("cmd", choices=("check", "capture", "real"))
    ap.add_argument("what", nargs="?", choices=("capture", "check"), help="real's: capture or check")
    ap.add_argument("--only", help="sets, comma-separated")
    ap.add_argument("--skip", help="sets left out, comma-separated")
    ap.add_argument("--exps", help="real's experiments, comma-separated (default: chosen, see the docs)")
    ap.add_argument("--out", default=str(REPO / "target" / "simlab" / "golden"))
    a = ap.parse_args()
    # Each line as it's printed, to a file or a pipe too.
    sys.stdout.reconfigure(line_buffering=True)
    split = lambda s: [x.strip() for x in (s or "").split(",") if x.strip()]
    if a.cmd == "real":
        if not a.what:
            sys.exit("real needs capture or check")
        sys.exit(0 if real(a.what, split(a.exps), a.out) else 1)
    names = [m for n in split(a.only) for m in ALIASES.get(n, (n,))] or list(CAPTURED if a.cmd == "capture"
                                                                            else SETS)
    skip = {m for n in split(a.skip) for m in ALIASES.get(n, (n,))}
    unknown = set(names) - set(SETS) | skip - set(SETS)
    if unknown:
        sys.exit(f"no set {', '.join(sorted(unknown))}: {', '.join(SETS)}")
    names = [n for n in names if n not in skip]
    if a.cmd == "capture":
        for n in names:
            if n not in CAPTURED:
                sys.exit(f"{n} has no golden of its own to capture")
            {"cli": golden_cli.capture, "matrices": golden_suite.capture}[n]()
        return
    if not a.only:
        print("golden: the whole check takes about 40 minutes; each of its groups takes under 15: "
              + ", ".join("--only " + ",".join(g) for g in GROUPS), file=sys.stderr)
    failed = [n for n in names if not run_set(n)]
    print(f"golden: {len(names) - len(failed)} of {len(names)} sets pass" + (f"; failed: {', '.join(failed)}"
                                                                              if failed else ""))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
