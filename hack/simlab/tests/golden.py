#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Every golden of the sims' results tools, in one command: what each
script prints must stay what it printed before simlab took its work over.

  python3 hack/simlab/tests/golden.py check [--only SET,...] [--skip SET,...]
  python3 hack/simlab/tests/golden.py capture [--only SET,...]
  python3 hack/simlab/tests/golden.py real capture|check [--exps E,...] [--out DIR]

The sets, in order (check runs them all, and exits 1 if any fails):

  cli           simdb.py's commands and analyze.py's verdict commands over
                a fixture cache, against testdata/golden/cli.txt.gz
                (golden_cli.py)
  matrices      suite.py farm --no-run's matrices and index for every tier,
                byte for byte, against testdata/golden/matrices.txt.gz
                (golden_suite.py)
  suite_test    hack/sim-suite/suite_test.py: the scorecards and arm scores
                of its fixtures, as the Python scoring printed them
  simdb_test    hack/sim-db/simdb_test.py
  engine        simlab's own tests (test_*.py here), under uv
  scorecards    hack/sim-suite/golden.py check: the latest farm
                experiments' scorecards and arm scores, the macros against
                the Python scoring at its REFERENCE, over the catalog (about
                6 minutes)

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
import subprocess
import sys
import time
from pathlib import Path

import golden_cli
import golden_suite
import goldens

HACK = Path(__file__).resolve().parents[2]
REPO = HACK.parent
SIMDB, ANALYZE = golden_cli.SIMDB, golden_cli.ANALYZE
SETS = ("cli", "matrices", "suite_test", "simdb_test", "engine", "scorecards")
CAPTURED = ("cli", "matrices")
sys.path.insert(0, str(HACK))
from simlab import bootstrap  # noqa: E402

UV = [bootstrap.uv() or "uv", "run", "--project", str(HACK), "--locked", "--quiet", "python"]


def script(args, label):
    """Whether a command exits 0; its output's last line, either way."""
    t0 = time.time()
    p = subprocess.run(args, cwd=REPO, stdin=subprocess.DEVNULL, capture_output=True, text=True)
    tail = (p.stdout + p.stderr).strip().splitlines()[-1:] or [""]
    print(f"{label}: {'ok' if p.returncode == 0 else 'FAILED'} ({time.time() - t0:.0f} s): {tail[0]}")
    if p.returncode:
        print(p.stdout[-4000:] + p.stderr[-4000:])
    return p.returncode == 0


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
        return script([*UV, "-m", "unittest", "discover", "-s", str(Path(__file__).parent), "-p", "test_*.py",
                       "-t", str(Path(__file__).parent)], name)
    if name == "scorecards":
        return script([sys.executable, str(HACK / "sim-suite" / "golden.py"), "check"], name)
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
                "AND len(arm_labels) >= 2 ORDER BY started_at DESC LIMIT 2")
    local = lake("SELECT e.exp_id FROM experiments e JOIN experiment_jobs j USING (exp_id) LEFT JOIN availability a "
                 "ON a.k = j.k AND a.tbl = 'reads' WHERE e.kind = 'local' GROUP BY e.exp_id, e.started_at "
                 "HAVING count(DISTINCT j.k) = count(DISTINCT j.k) FILTER (WHERE a.state = 'local') "
                 "AND count(DISTINCT j.k) BETWEEN 4 AND 40 ORDER BY e.started_at DESC LIMIT 1")
    return [r["exp_id"] for r in farm + local]


def real_invocations(exp, kind, matrices, src):
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
    if kind == "farm":
        out += [("collapse", ANALYZE, ["collapse", *matrices]), ("map", ANALYZE, ["map", *matrices]),
                ("track", ANALYZE, ["track", *matrices])]
    else:
        out += [("pooled", SIMDB, ["pooled", *src]), ("pooled-phase", SIMDB, ["pooled", "--by", "phase", *src]),
                ("aligned-first-read", SIMDB, ["aligned", "first_read", "--per", "node", "--field", "lag_ms",
                                               "--after", "10", *src]),
                ("reads", SIMDB, ["sql", "SELECT r.cell, r.arm, count(*) AS n, quantile_cont(lag_ms, 0.99) AS p99 "
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
        for name, path, args in real_invocations(exp, kind, matrices, src):
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
    split = lambda s: [x.strip() for x in (s or "").split(",") if x.strip()]
    if a.cmd == "real":
        if not a.what:
            sys.exit("real needs capture or check")
        sys.exit(0 if real(a.what, split(a.exps), a.out) else 1)
    names = split(a.only) or list(CAPTURED if a.cmd == "capture" else SETS)
    unknown = set(names) - set(SETS) | set(split(a.skip)) - set(SETS)
    if unknown:
        sys.exit(f"no set {', '.join(sorted(unknown))}: {', '.join(SETS)}")
    names = [n for n in names if n not in split(a.skip)]
    if a.cmd == "capture":
        for n in names:
            if n not in CAPTURED:
                sys.exit(f"{n} has no golden of its own to capture")
            {"cli": golden_cli.capture, "matrices": golden_suite.capture}[n]()
        return
    failed = [n for n in names if not run_set(n)]
    print(f"golden: {len(names) - len(failed)} of {len(names)} sets pass" + (f"; failed: {', '.join(failed)}"
                                                                              if failed else ""))
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
