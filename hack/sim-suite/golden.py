#!/usr/bin/env python3
# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The catalog's scoring macros (hack/sim-db/scoring.sql) against the
Python scoring suite.py had before them: what both print for the latest
farm experiments must be the same, line for line.

  hack/sim-suite/golden.py check [--exps E,...] [--n N] [--local N] [--ref REV]
  hack/sim-suite/golden.py fixtures [--exps E,...] [--n N] [--ref REV]

`check` scores, for each experiment, every arm against its base three
ways and compares what they print: the reference (suite.py at REV,
REFERENCE by default, scoring its rows in Python), this checkout's
suite.py (the macros over the sides it reads from the farm's indexes),
and the macros over the experiment's arms as the catalog's
experiment_jobs has them (arm_side). Each prints the arm score, ranked by
lag and for overhead (with no budget, and with one of 1.05), and the
scorecard, every metric and the default's. The reference's t table is the
macros' (t975), so the two differ by nothing but their code. The
experiments are the latest N of each kind (--refs and --sets runs) with
two arms or more; an experiment's base is its index's, else the arm the
farm reused from an earlier run (an A/B's baseline), else its first.

`fixtures` writes suite_test.py's fixtures (testdata/golden/): for each
experiment the cells that exercise the most (a collapse, a broken
guardrail, the most marks), the catalog's tables of their runs (Parquet:
rows and verdicts with the columns METRICS reads, jobs, raw.runs,
experiment_jobs, experiment_cells), the sides, and what the reference
prints over those cells alone; and a made-up experiment (SYNTHETIC) for
what the farm's rarely show: one seed, zeros, an 18-seed cell, ties,
guardrails and the overhead ceiling.
"""

import argparse
import collections
import contextlib
import difflib
import gzip
import importlib.util
import io
import json
import math
import re
import subprocess
import sys
import types
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
# The last suite.py that scored in Python.
REFERENCE = "4eee6065"
WORK = ROOT / "target" / "golden"
FIXTURES = Path(__file__).with_name("testdata") / "golden"
RANKS = {"lag": (None,), "overhead": (None, 1.05)}


def load(path, name):
    spec = importlib.util.spec_from_file_location(name, path)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


_suite = None


def suite():
    """This checkout's suite.py."""
    global _suite
    if _suite is None:
        _suite = load(Path(__file__).with_name("suite.py"), "suite")
    return _suite


def reference(rev):
    """suite.py at `rev`, its t table the macros' (t975)."""
    WORK.mkdir(parents=True, exist_ok=True)
    path = WORK / f"suite_{rev}.py"
    path.write_text(subprocess.run(["git", "show", f"{rev}:hack/sim-suite/suite.py"], cwd=ROOT, check=True,
                                   capture_output=True, text=True).stdout)
    ref = load(path, "suite_ref")
    ref._simdb = suite().simdb()
    ref.T975 = {r["df"]: r["t"] for r in suite().score_query(
        "SELECT df, t975_at(df) AS t FROM range(1, 1001) r(df) ORDER BY df;")}
    return ref


def printed(fn):
    out = io.StringIO()
    with contextlib.redirect_stdout(out):
        fn()
    return out.getvalue()


def rank(kind, budget):
    return types.SimpleNamespace(rank=kind, lag_budget=budget,
                                 lag_ceiling=float(suite().analyze().DELIVERY_TARGET_MS))


def experiments(n, names=None):
    """The experiments `check` scores: [{exp, cells, knob, base, arms}]."""
    s = suite()
    where = f"exp_id IN ({', '.join(s.q(e) for e in names)})" if names else "len(arm_labels) >= 2"
    got = s.catalog().lake(f"""
WITH e AS (SELECT exp_id, knob IS NOT NULL AS sets, base, arm_labels, started_at FROM experiments
           WHERE kind = 'farm' AND NOT pending AND {where}),
r AS (SELECT exp_id, arm_label, count(*) FILTER (WHERE reused) AS reused FROM experiment_jobs GROUP BY ALL),
x AS (SELECT e.exp_id, any_value(e.sets) AS sets, any_value(e.base) AS base, any_value(e.arm_labels) AS arm_labels,
             arg_max(r.arm_label, r.reused) AS reused_arm, max(r.reused) AS most_reused,
             any_value(e.started_at) AS started_at
      FROM e JOIN r USING (exp_id) GROUP BY e.exp_id)
SELECT *, row_number() OVER (PARTITION BY sets ORDER BY started_at DESC) AS nth FROM x ORDER BY started_at DESC;""")
    out = []
    for e in got:
        if not names and e["nth"] > n:
            continue
        index = json.loads(s.farm_index(e["exp_id"]).read_text())
        labels = e["arm_labels"]
        base = (e["base"] if e["base"] in labels else e["reused_arm"] if e["most_reused"] else labels[0])
        out.append({"exp": e["exp_id"], "cells": index["cells"], "knob": index.get("knob"), "base": base,
                    "arms": [a for a in labels if a != base]})
    return out


def catalog_sides(exp, arms):
    """`exp`'s arms as the catalog's arm_side gives them."""
    s = suite()
    out = {}
    for arm in arms:
        rows = s.score_query(f"SELECT unnest(arm_side({s.q(exp)}, {s.q(arm)}), recursive := true);")
        out[arm] = {(r["cell"], r["seed"]): (r["k"], r["src"]) for r in rows}
    return out


def texts(mod, cells, base_rows, arm_rows, exp, base, arm):
    """What `mod` (a suite.py) prints scoring `arm_rows` against
    `base_rows` over `cells`: {name: text}."""
    out = {}
    for kind, budgets in RANKS.items():
        for budget in budgets:
            r = rank(kind, budget)
            out[f"arm {kind} {budget}"] = printed(lambda: mod.print_arm_score(
                f"{exp}:{arm} against {exp}:{base}", mod.arm_score(cells, base_rows, arm_rows, r)))
    for show_all in (False, True):
        out[f"scorecard all={show_all}"] = printed(lambda: mod.scorecard(
            cells, base_rows, arm_rows, f"{exp}:{base}", f"{exp}:{arm}", show_all))
    return out


def compare(name, want, got):
    """The lines `got` differs from `want` by, printed; whether none."""
    if want == got:
        return True
    diff = list(difflib.unified_diff(want.splitlines(), got.splitlines(), "reference", name, lineterm="", n=1))
    print("\n".join(diff[:60]) + (f"\n... {len(diff) - 60} more" if len(diff) > 60 else ""))
    return False


def stats(text):
    """A scorecard's cells, lines with a mark, and totals line."""
    cells = sum(1 for line in text.splitlines() if re.match(r"^\S.*, \d+ seeds \(", line))
    marked = sum(1 for line in text.splitlines() if re.match(r"^   [-+~h] ", line))
    moved = next((line for line in text.splitlines() if line.startswith("moved:")), "")
    return cells, marked, moved


def local_cases(n):
    """The latest `n` local suite invocations that ran both the base and
    the working tree (`run`): their cells, and each arm's runs as `run`
    orders them (by label, then cell, then seed), base labels against tree
    ones."""
    s = suite()
    got = s.catalog().lake(f"""
SELECT exp_id FROM experiments WHERE kind = 'local' AND len(list_filter(arm_labels, lambda x: x LIKE 'tree-%')) > 0
  AND len(list_filter(arm_labels, lambda x: x NOT LIKE 'tree-%')) > 0
ORDER BY started_at DESC LIMIT {int(n)};""")
    out = []
    for e in got:
        jobs = s.catalog().lake(f"SELECT cell, arm_label, seed, k, src FROM experiment_jobs WHERE exp_id = "
                                f"{s.q(e['exp_id'])} ORDER BY arm_label, cell, seed;")
        names = sorted({j["cell"] for j in jobs})
        cells = [{"name": c, "why": "local"} for c in names]
        order = {c: i for i, c in enumerate(names)}
        sides = {"base": {}, "tree": {}}
        for j in sorted(jobs, key=lambda j: (j["arm_label"], order[j["cell"]], j["seed"])):
            side = "tree" if j["arm_label"].startswith("tree-") else "base"
            sides[side][(j["cell"], j["seed"])] = (j["k"], j["src"])
        out.append({"exp": e["exp_id"], "cells": cells, "sides": sides})
    return out


def check_local(ref, n):
    """`check` over local invocations (local_cases): the reference over
    simdb.rows' rows of the runs, as `run` scored them."""
    ok = True
    for e in local_cases(n):
        rows = ref.simdb().rows([v[::-1] for side in e["sides"].values() for v in side.values()])
        ref_rows = {n_: {at: rows[(src, k)] for at, (k, src) in side.items() if (src, k) in rows}
                    for n_, side in e["sides"].items()}
        want = texts(ref, e["cells"], ref_rows["base"], ref_rows["tree"], e["exp"], "base", "tree")
        got = texts(suite(), e["cells"], e["sides"]["base"], e["sides"]["tree"], e["exp"], "base", "tree")
        same = all(compare(f"local {e['exp']} {k}", want[k], got[k]) for k in want)
        ok &= same
        cells, marked, moved = stats(want["scorecard all=False"])
        print(f"{e['exp']} tree against base: {'same' if same else 'DIFFERENT'}; {cells} cells, {marked} marked "
              f"lines, {moved}")
        sys.stdout.flush()
    return ok


def check(a):
    ref = reference(a.ref)
    ok = True
    if a.local:
        ok &= check_local(ref, a.local)
    for e in experiments(a.n, a.exps) if a.n or a.exps else ():
        ref_rows = ref.farm_rows(e["exp"], e["cells"], e["knob"])
        new = suite().farm_rows(e["exp"], e["cells"], e["knob"])
        cat = catalog_sides(e["exp"], [e["base"], *e["arms"]])
        for arm in e["arms"]:
            want = texts(ref, e["cells"], ref_rows[e["base"]], ref_rows[arm], e["exp"], e["base"], arm)
            for how, sides in (("suite.py", new), ("arm_side", cat)):
                got = texts(suite(), e["cells"], sides[e["base"]], sides[arm], e["exp"], e["base"], arm)
                same = all(compare(f"{how} {e['exp']}:{arm} {k}", want[k], got[k]) for k in want)
                ok &= same
                cells, marked, moved = stats(want["scorecard all=False"])
                breaks = want["arm lag None"].splitlines()[1].split("guardrails ", 1)[-1]
                print(f"{e['exp']} {arm} against {e['base']} ({how}): {'same' if same else 'DIFFERENT'}; "
                      f"{cells} cells, {marked} marked lines, {moved}; guardrails {breaks[:100]}")
                sys.stdout.flush()
    sys.exit(0 if ok else 1)


# --- fixtures -------------------------------------------------------------------

# A made-up experiment: cells of rows and verdicts, each (name, why,
# [(seed, base row, arm row)]), a row's fields as METRICS read them and its
# verdict's `collapsed`.
def synthetic():
    def row(lag, unhealthy=0.0, collapsed=False, wall=1.0, **f):
        return {"lag_p99_ms": lag, "lag_p50_ms": lag / 2, "delivery": f.pop("delivery", 1.0), "_unhealthy": unhealthy,
                "_collapsed": collapsed, "_wall_s": wall, "_rss_mb": f.pop("rss", 100.0), **f}
    cells = [
        ("one-seed", "one seed: an interval of nothing", [(1, row(100), row(200))]),
        ("zeros", "ratios over zeros, a share falling", [(s, row(0 if s % 2 else 50, timeouts=0, wasted=1,
                                                              chunks_read=10),
                                                       row(60, timeouts=s, wasted=2, chunks_read=10))
                                                      for s in range(1, 9)]),
        ("eighteen", "18 seeds: the t table between 16 and 20",
         [(s, row(100 + s, unhealthy=0.01 * s), row(130 + 2 * s, unhealthy=0.02 * s, wall=1.5)) for s in range(1, 19)]),
        ("ties", "every pair equal", [(s, row(300, depth_1=3, depth_2=1), row(300, depth_1=3, depth_2=1))
                                      for s in range(1, 5)]),
        ("collapse", "the arm collapses where the base doesn't",
         [(s, row(800 + s, delivery=0.99), row(5000 + 10 * s, unhealthy=0.5, collapsed=s <= 6, delivery=0.9 - 0.01 * s))
          for s in range(1, 9)]),
        ("ceiling", "lag p99 past the overhead ceiling",
         [(s, row(1000 + 10 * s, wire_per_useful=1.3, lag_p99_ms_weak=900 + s, lag_p99_ms_hostile=700 + s,
                  lag_p99_ms_honest=1000 + 10 * s),
           row(1600 + 30 * s, wire_per_useful=1.1, lag_p99_ms_weak=1500 + 3 * s, lag_p99_ms_hostile=1000 + s,
               lag_p99_ms_honest=1600 + 30 * s)) for s in range(1, 9)]),
    ]
    return cells


def synthetic_case():
    """SYNTHETIC as a case: its tables' records, its cells and sides, and
    the reference's rows."""
    cells, recs, ref_rows, sides = [], collections.defaultdict(list), {"base": {}, "arm": {}}, {"base": {}, "arm": {}}
    for name, why, seeds in synthetic():
        cells.append({"name": name, "sim": "ppspp", "test": "synthetic", "why": why})
        for seed, *pair in seeds:
            for arm, r in zip(("base", "arm"), pair):
                k, src = f"syn:{arm}:{name}:{seed}", "suite/synthetic/0"
                fields = {f: v for f, v in r.items() if not f.startswith("_")}
                recs["rows"].append({"k": k, "src": src, **fields})
                recs["verdicts"].append({"k": k, "src": src, "collapsed": r["_collapsed"], "unhealthy": r["_unhealthy"]})
                recs["jobs"].append({"k": k, "src": src, "trace": f"t{seed}", "reads_hash": ""})
                recs["runs"].append({"k": k, "src": src, "cell": name, "arm": arm, "seed": seed, "trace": f"t{seed}",
                                     "wall_s": r["_wall_s"], "rss_mb": r["_rss_mb"]})
                recs["experiment_jobs"].append({"exp_id": "synthetic", "cell": name, "arm_label": arm, "seed": seed,
                                                "k": k, "src": src, "flush": src, "matrix": f"synthetic-{len(cells):02d}"})
                healthy = [True] * 20 + ([False] * 10 if r["_collapsed"] else [True] * 10)
                ref_rows[arm][(name, seed)] = {**fields, "trace": f"t{seed}", "_wall_s": r["_wall_s"],
                                               "_rss_mb": r["_rss_mb"], "_metrics": {},
                                               "_verdict": {"unhealthy": r["_unhealthy"], "healthy": healthy,
                                                            "startup": 15}}
                sides[arm][(name, seed)] = (k, src)
        recs["experiment_cells"].append({"exp_id": "synthetic", "cell": name, "why": why,
                                         "matrix": f"synthetic-{len(cells):02d}"})
    return cells, recs, ref_rows, sides


def pick_cells(text_arm, text_card, cells):
    """Two cells of a scorecard that exercise the most: the first broken
    guardrail's, else the one with the most collapses; and the one with the
    most marked lines."""
    marks, collapses, cur = collections.Counter(), collections.Counter(), None
    for line in text_card.splitlines():
        m = re.match(r"^(\S.*), \d+ seeds \(", line)
        if m:
            cur = m.group(1)
        elif cur and re.match(r"^   [-+~h] ", line):
            marks[cur] += 1
        if cur and "collapsed" in line and "/" in line:
            m = re.search(r"(\d+)/\d+\s+(\d+)/\d+", line)
            if m:
                collapses[cur] = int(m.group(1)) + int(m.group(2))
    names = [c["name"] for c in cells]
    broke = re.search(r"guardrails (\S+)", text_arm.splitlines()[1])
    want = []
    if broke and broke.group(1) in names:
        want.append(broke.group(1))
    for counter in ((marks,) if want else (collapses, marks)):
        for name, _ in counter.most_common():
            if name not in want:
                want.append(name)
                break
    return [c for c in cells if c["name"] in want]


def fixtures(a):
    s, ref = suite(), reference(a.ref)
    FIXTURES.mkdir(parents=True, exist_ok=True)
    cases, runs = [], set()
    for e in experiments(a.n, a.exps):
        ref_rows = ref.farm_rows(e["exp"], e["cells"], e["knob"])
        new = s.farm_rows(e["exp"], e["cells"], e["knob"])
        arm = e["arms"][0]
        full = texts(ref, e["cells"], ref_rows[e["base"]], ref_rows[arm], e["exp"], e["base"], arm)
        cells = pick_cells(full["arm lag None"], full["scorecard all=False"], e["cells"])
        names = {c["name"] for c in cells}
        sub = lambda rows: {at: r for at, r in rows.items() if at[0] in names}
        want = texts(ref, cells, sub(ref_rows[e["base"]]), sub(ref_rows[arm]), e["exp"], e["base"], arm)
        sides = {n: sub(new[n]) for n in (e["base"], arm)}
        runs |= {v for side in sides.values() for v in side.values()}
        cases.append({"exp": e["exp"], "base": e["base"], "arm": arm, "cells": cells, "want": want,
                      "sides": {n: [[c, seed, k, src] for (c, seed), (k, src) in side.items()]
                                for n, side in sides.items()}})
        print(f"{e['exp']}: {', '.join(sorted(names))}", file=sys.stderr)
    # The catalog's tables of those runs: the columns METRICS reads.
    used = collections.defaultdict(set)
    for _, sql, *_ in s.METRICS:
        for t, f in re.findall(r"\b([rvj])\.([a-z_][a-z0-9_]*)\b", sql):
            used[t].add(f)
    have = {r["t"]: set(r["c"]) for r in s.catalog().lake(
        "SELECT table_name AS t, list(column_name) AS c FROM duckdb_columns() WHERE database_name = "
        "current_database() AND schema_name = 'main' AND table_name IN ('rows', 'verdicts') GROUP BY ALL;")}
    rcols = ", ".join(s.catalog().ident(c) for c in sorted(used["r"] & have["rows"]))
    vcols = ", ".join(s.catalog().ident(c) for c in sorted((used["v"] | {"collapsed"}) & have["verdicts"]))
    pairs = ", ".join(f"({s.q(k)}, {s.q(src)})" for k, src in sorted(runs))
    ks = f"(SELECT DISTINCT k FROM (VALUES {pairs}) w(k, src))"
    exps = ", ".join(s.q(c["exp"]) for c in cases)
    queries = {
        "rows": f"SELECT k, src, {rcols} FROM rows WHERE k IN {ks}",
        "verdicts": f"SELECT k, src, {vcols} FROM verdicts WHERE k IN {ks}",
        "jobs": f"SELECT k, src, trace, reads_hash FROM jobs WHERE k IN {ks}",
        "runs": f"SELECT r.k, r.src, r.cell, r.arm, r.seed, r.trace, r.wall_s, r.rss_mb FROM raw.runs r "
                f"JOIN (SELECT k, src FROM (VALUES {pairs}) w(k, src) UNION SELECT k, src FROM jobs WHERE k IN {ks}) w "
                f"USING (k, src) QUALIFY row_number() OVER (PARTITION BY r.k, r.src ORDER BY r.cell) = 1",
        "experiment_jobs": f"SELECT exp_id, cell, arm_label, seed, k, src, flush, matrix FROM experiment_jobs "
                           f"WHERE exp_id IN ({exps}) AND k IN {ks}",
        "experiment_cells": f"SELECT exp_id, cell, why, matrix FROM experiment_cells WHERE exp_id IN ({exps})",
    }
    syn_cells, syn_recs, syn_ref, syn_sides = synthetic_case()
    syn_want = texts(ref, syn_cells, syn_ref["base"], syn_ref["arm"], "synthetic", "base", "arm")
    cases.append({"exp": "synthetic", "base": "base", "arm": "arm", "cells": syn_cells, "want": syn_want,
                  "sides": {n: [[c, seed, k, src] for (c, seed), (k, src) in side.items()]
                            for n, side in syn_sides.items()}})
    for t, sql in queries.items():
        path = FIXTURES / f"{t}.parquet"
        syn = WORK / f"synthetic-{t}.jsonl"
        syn.write_text("".join(json.dumps(r) + "\n" for r in syn_recs.get(t, [])))
        both = (f"SELECT * FROM ({sql}) UNION ALL BY NAME SELECT * FROM read_json({s.q(str(syn))}, "
                f"format = 'newline_delimited')" if syn_recs.get(t) else sql)
        s.catalog().lake(f"COPY ({both} ORDER BY ALL) TO {s.q(str(path))} (FORMAT parquet, COMPRESSION zstd);",
                         out=None)
    (FIXTURES / "golden.json.gz").write_bytes(gzip.compress(
        (json.dumps({"reference": a.ref, "cases": cases}, indent=1) + "\n").encode(), mtime=0))
    size = sum(p.stat().st_size for p in FIXTURES.iterdir())
    print(f"{len(cases)} cases, {len(runs)} runs, {size / 1024:.0f} KiB in {FIXTURES}", file=sys.stderr)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    for name in ("check", "fixtures"):
        p = sub.add_parser(name)
        p.add_argument("--exps", type=lambda x: [e.strip() for e in x.split(",") if e.strip()],
                       help="experiments, comma-separated (default: the latest of each kind)")
        p.add_argument("--n", type=int, default=5, help="the latest experiments of each kind")
        p.add_argument("--ref", default=REFERENCE, help="the reference suite.py's commit")
    sub.choices["check"].add_argument("--local", type=int, default=0,
                                      help="also the latest N local invocations of `run`")
    a = ap.parse_args()
    {"check": check, "fixtures": fixtures}[a.cmd](a)


if __name__ == "__main__":
    main()
