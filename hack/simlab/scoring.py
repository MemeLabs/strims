# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The catalog's scoring macros (hack/simlab/sql/scoring.sql) over two
arms' runs, as scorecards and arm scores, each of which prints what
suite.py prints (`text`) and goes to the console as JSON (`json`).

A side is an arm's runs, {(cell, seed): (k, src)}: the catalog's job k
for each cell and seed, and the source of the run's own copy (its farm
index line's flush, its local invocation's), whose host numbers it reads.
simlab.suite's run_cells and farm_rows give sides, and so does `sides`, an
experiment's arms as the catalog's experiment_jobs has them (arm_side). A
cell is {name, why}. Every query runs in a session of its own holding the
scoring objects as simlab.suite defines its METRICS
(catalog.scoring_sql, TEMP), so a branch's METRICS score its runs, over
the catalog, or over SCORE_DB.

  query(sql) -> [row]
                 the rows of `sql` (the CLI's JSON values) in such a
                 session
  scorecard(cells, a, b, a_label, b_label, show_all=False, traces=True)
                 -> Scorecard
                 side B against side A over `cells`: per cell, the collapse
                 test and each metric's paired effect, the marks' totals
                 and how many runs B ran as A did
  arm_score(cells, base, arm, title=None, rank="lag", lag_budget=None,
            lag_ceiling=None) -> ArmScore
                 an arm against the base on the same seeds: collapses,
                 lag p99 and wire per useful byte, the guardrails it broke
                 and its rank key (ranked by lag, or for overhead)
  sides(exp, arms) -> {arm: side}
  cells_of(exp_a, exp_b=None, only=None) -> [cell]
                 the cells of exp_b (exp_a) that exp_a has too, in exp_b's
                 order, `only` those named
  score(exp_a, arm_a, exp_b, arm_b, cells=None, show_all=False,
        traces=True) -> Scorecard
                 arm_b of exp_b against arm_a of exp_a over their cells, as
                 `suite.py score exp_a:arm_a exp_b:arm_b` prints it
  arm_scores(exp, base, arms=None, cells=None, rank="lag",
             lag_budget=None, lag_ceiling=None) -> [ArmScore]
                 each arm of `exp` (every one but `base` by default)
                 against `base`, best first
  structural(exp) -> [row]
                 the cells where every arm collapsed at every seed
  dynamics(exp) -> [row]
                 each arm and cell's lag by phase, spikes, stalls, startup
                 and unhealthy spells

The catalog's queries need the duckdb module (simlab.bootstrap.engine);
SCORE_DB's run in store_host.duck (the CLI, or its executor).
"""

import collections
import dataclasses
import math

from . import catalog, engine, store_host, verdict

q = store_host.q

# The DuckDB file scores are computed over, None for the catalog: a file
# holding the tables the scoring reads (rows, verdicts, jobs, raw.runs,
# experiment_jobs, experiment_cells), as the tests' fixtures do.
SCORE_DB = None


def query(sql):
    """The rows of `sql` over the catalog (or SCORE_DB) in a session holding
    the scoring objects as simlab.suite defines them (catalog.scoring_sql,
    TEMP: its METRICS, whatever the catalog's indexer had); a macro that
    reads a table the catalog's indexer didn't make fails saying so, and
    the others still score."""
    run = (lambda s: catalog.lake(s)) if SCORE_DB is None else (lambda s: store_host.duck(s, db=SCORE_DB))
    cols = {r["t"]: set(r["c"]) for r in run(
        "SELECT table_name AS t, list(column_name) AS c FROM duckdb_columns() WHERE database_name = "
        "current_database() AND schema_name = 'main' GROUP BY ALL;")}
    return run(catalog.scoring_sql(catalog.scoring_cols(cols, have=cols), temp=True) + "\n" + sql)


def sides_sql(**sides):
    """SQL making a TEMP table of each side, `name`: one row, `s` the side as
    scoring.sql takes one (in its order)."""
    out = []
    for name, side in sides.items():
        items = ", ".join(f"{{'cell': {q(c)}, 'seed': {int(s)}, 'k': {q(k)}, 'src': {q(src)}}}"
                          for (c, s), (k, src) in side.items())
        out.append(f"CREATE OR REPLACE TEMP TABLE {name} AS SELECT "
                   f"[{items}]::STRUCT(cell VARCHAR, seed BIGINT, k VARCHAR, src VARCHAR)[] AS s;")
    return "\n".join(out) + "\n"


def cell_list(cells):
    return "[" + ", ".join(q(c["name"]) for c in cells) + "]::VARCHAR[]"


def plain(x):
    """`x` with its non-finite floats as the console's JSON gives them
    ("NaN", "Infinity", "-Infinity"), in lists and dicts too."""
    if isinstance(x, float):
        return engine.float_json(x)
    if isinstance(x, dict):
        return {k: plain(v) for k, v in x.items()}
    if isinstance(x, (list, tuple)):
        return [plain(v) for v in x]
    return x


# --- scorecards -------------------------------------------------------------


def fmt(x):
    if x is None:
        return "-"
    if isinstance(x, float) and abs(x) < 10:
        return f"{x:.3g}"
    return f"{x:.0f}"


def effect_text(e):
    """An effect (effects_of's) as the scorecard prints it."""
    if e["effect"] is None:
        return "-"
    if e["kind"] == "ratio":
        return f"x{e['effect']:.2f} [{e['lo']:.2f},{e['hi']:.2f}]"
    return f"{e['effect']:+.3f} [{e['lo']:+.3f},{e['hi']:+.3f}]"


@dataclasses.dataclass
class Scorecard:
    """Side B against side A over cells: each cell's name and why, its
    effects (effects_of's rows, in METRICS' order) and its collapse test
    (collapse_test_of's row, None where it has none), and how many runs B
    ran as A did in each cell the sides share (traces_of's rows). The text
    shows each cell's moved metrics and SHOWN's (every one with
    `show_all`), and the traces with `show_traces`."""
    a_label: str
    b_label: str
    cells: list = dataclasses.field(default_factory=list)
    traces: list = dataclasses.field(default_factory=list)
    show_all: bool = False
    show_traces: bool = True

    def totals(self):
        """The marks' counts, {mark: n}: every effect but the host metrics',
        and every collapse test that has one."""
        out = collections.Counter()
        for c in self.cells:
            out.update(e["mark"] for e in c["effects"] if not e["host"])
            if c["collapse"] and c["collapse"]["mark"] is not None:
                out[c["collapse"]["mark"]] += 1
        return out

    def text(self):
        out = [f"A = {self.a_label}   B = {self.b_label}",
               "legend: + better  - worse  ~ moved  (ratio B/A or difference B-A, 95 % t-interval over seeds)"]
        for c in self.cells:
            lines = [f"   {e['mark']} {e['metric']:20} {fmt(e['a_median']):>9} {fmt(e['b_median']):>9}  "
                     f"{effect_text(e)}" for e in c["effects"]
                     if e["mark"] != " " or e["shown"] or self.show_all]
            # Whether a run collapses is near all-or-nothing, and a change
            # redraws which seeds do: the counts, not the pairs, carry it.
            t = c["collapse"] or {"ka": 0, "kb": 0, "n": 0, "mark": None}
            ka, kb, n = t["ka"], t["kb"], t["n"]
            if t["mark"] is not None:
                lines.insert(0, f"   {t['mark']} {'collapsed':20} {ka:>7}/{n} {kb:>7}/{n}  Fisher p={t['p']:.2f}")
            out.append(f"\n{c['name']}, {n} seeds ({c['why']})")
            out.append("\n".join(lines) if lines else "   (no rows)")
        totals = self.totals()
        out.append(f"\nmoved: {totals['+']} better, {totals['-']} worse, {totals['~']} changed; {totals[' ']} ties")
        total = sum(t["runs"] for t in self.traces)
        if self.show_traces and total:
            same = sum(t["same"] for t in self.traces)
            differ = sorted((t for t in self.traces if t["same"] < t["runs"]), key=lambda t: t["same"] - t["runs"])
            cells = ", ".join(f"{t['cell']} ({t['runs'] - t['same']})" for t in differ[:8])
            more = f" and {len(differ) - 8} more" if len(differ) > 8 else ""
            out.append(f"traces: {same} of {total} runs as A's" + (f"; differ in {cells}{more}" if 0 < same < total
                                                                   else ""))
        return "".join(line + "\n" for line in out)

    def json(self):
        totals = self.totals()
        return plain({"a": self.a_label, "b": self.b_label,
                      "cells": [{"name": c["name"], "why": c["why"], "collapse": c["collapse"],
                                 "effects": c["effects"]} for c in self.cells],
                      "moved": {"better": totals["+"], "worse": totals["-"], "changed": totals["~"],
                                "ties": totals[" "]},
                      "traces": self.traces})


def scorecard(cells, a, b, a_label, b_label, show_all=False, traces=True):
    """Side `b` against side `a` over `cells` (effects_of, collapse_test_of
    and traces_of)."""
    cl, ab = cell_list(cells), "(SELECT s FROM side_a), (SELECT s FROM side_b)"
    got = query(sides_sql(side_a=a, side_b=b) + f"""
SELECT (SELECT list(e ORDER BY cell_ord, ord) FROM effects_of({cl}, {ab}) e) AS effects,
       (SELECT list(c) FROM collapse_test_of({cl}, {ab}) c) AS collapses,
       (SELECT list(t ORDER BY a_first) FROM traces_of({ab}) t) AS traces;""")[0]
    effects = collections.defaultdict(list)
    for e in got["effects"] or []:
        effects[e["cell"]].append(e)
    tests = {c["cell"]: c for c in got["collapses"] or []}
    return Scorecard(a_label, b_label, [{"name": c["name"], "why": c["why"], "effects": effects.get(c["name"], []),
                                         "collapse": tests.get(c["name"])} for c in cells],
                     got["traces"] or [], show_all, traces)


# --- arm scores -------------------------------------------------------------


def break_text(b):
    """A guardrail an arm broke (arm_cells_of's), as the arm score names it."""
    if b["kind"] == "collapsed":
        return f"{b['cell']} collapsed {b['ka']}->{b['kb']}/{b['n']}"
    if b["kind"] == "lag_ceiling":
        return f"{b['cell']} lag p99 {b['a']:.0f}->{b['b']:.0f} ms past {b['x']:.0f}"
    if b["kind"] == "worse":
        return f"{b['cell']} {b['metric']}"
    return f"{b['cell']} {b['metric']} x{b['x']:.2f}"


def ratio_text(x):
    return "-" if x is None else f"x{x:.3f}"


@dataclasses.dataclass
class ArmScore:
    """An arm against the base on the same seeds (arm_score_of and
    arm_cells_of): its collapsed runs and the base's, the geometric means
    over cells of its lag p99 ratio (p99) and its wire bytes per useful
    byte (wire), the guardrails it broke (GUARDRAILS; arm_cells_of's
    breaks), each cell's collapses and lag p99 ratio (arm_cells_of's rows),
    whether it was ranked for overhead, and its rank key: broken arms last,
    then fewer collapsed runs, then the lower lag p99 ratio, or wire per
    useful byte when ranked for overhead."""
    title: str
    collapsed: int
    base_collapsed: int
    p99: float
    wire: float
    breaks: list
    cells: list
    overhead: bool
    rank_key: tuple

    @property
    def broken(self):
        return [break_text(b) for b in self.breaks]

    def text(self):
        out = [f"\n{self.title}: collapsed {self.collapsed} ({self.base_collapsed}), p99 {ratio_text(self.p99)}, "
               f"wire/useful {ratio_text(self.wire)}, guardrails {'; '.join(self.broken) or 'ok'}"]
        for c in self.cells:
            lag = "-" if c["lag"] is None else f"x{c['lag']:.2f} [{c['lag_lo']:.2f},{c['lag_hi']:.2f}]"
            out.append(f"   {c['cell']:22} collapsed {c['ka']:>3} -> {c['kb']:>3} of {c['n']:<3} Fisher p={c['p']:.2f}"
                       f"  lag p99 {lag}")
        return "".join(line + "\n" for line in out)

    def json(self):
        return plain({"title": self.title, "collapsed": self.collapsed, "base_collapsed": self.base_collapsed,
                      "p99": self.p99, "wire": self.wire, "broken": self.broken, "breaks": self.breaks,
                      "cells": self.cells, "overhead": self.overhead, "rank_key": list(self.rank_key)})


def arm_score(cells, base, arm, title=None, rank="lag", lag_budget=None, lag_ceiling=None):
    """Side `arm` against side `base` over `cells`, ranked by lag or for
    overhead (`rank`). Ranked for overhead, a cell whose median lag p99 the
    base kept under `lag_ceiling` (the delivery target by default:
    verdict.DELIVERY_TARGET_MS) breaks a guardrail when the arm's isn't, and
    its lag ratios are guarded only by `lag_budget`, when one is given."""
    overhead = rank == "overhead"
    # No budget is an infinite one.
    budget = lag_budget if overhead and lag_budget is not None else math.inf
    ceiling = lag_ceiling if overhead and lag_ceiling is not None else verdict.DELIVERY_TARGET_MS
    opts = (f"overhead := {str(overhead).lower()}, lag_ceiling := {float(ceiling)!r}, "
            f"lag_budget := '{float(budget)!r}'::DOUBLE")
    cl, ab = cell_list(cells), "(SELECT s FROM side_a), (SELECT s FROM side_b)"
    got = query(sides_sql(side_a=base, side_b=arm) + f"""
SELECT (SELECT list(c ORDER BY cell_ord) FROM arm_cells_of({cl}, {ab}, {opts}) c) AS cells,
       (SELECT s FROM arm_score_of({cl}, {ab}, {opts}) s) AS score;""")[0]
    sc, key = got["score"], got["score"]["rank_key"]
    return ArmScore(title, sc["collapsed"], sc["base_collapsed"], sc["p99"], sc["wire"], sc["breaks"],
                    got["cells"] or [], overhead, (key["broken"], key["collapsed"], key["by"]))


# --- experiments ------------------------------------------------------------


def sides(exp, arms):
    """Experiment `exp`'s arms `arms` as sides (arm_side: experiment_jobs),
    {arm: side}."""
    out = {}
    for arm in arms:
        rows = query(f"SELECT unnest(arm_side({q(exp)}, {q(arm)}), recursive := true);")
        out[arm] = {(r["cell"], r["seed"]): (r["k"], r["src"]) for r in rows}
    return out


def cells_of(exp_a, exp_b=None, only=None):
    """The cells of `exp_b` (by default `exp_a`) that `exp_a` has too, in
    exp_b's order (exp_cells), but for those `only` names when given: [{name,
    why}]."""
    exp_b = exp_b or exp_a
    rows = query(f"""
WITH w AS (SELECT exp_cells({q(exp_a)}, {q(exp_b)}) AS l)
SELECT cell AS name, arg_min(why, matrix) AS why FROM experiment_cells, w
WHERE exp_id = {q(exp_b)} AND list_contains(w.l, cell) GROUP BY cell, w.l ORDER BY list_position(w.l, cell);""")
    return [r for r in rows if not only or r["name"] in set(only)]


def score(exp_a, arm_a, exp_b, arm_b, cells=None, show_all=False, traces=True):
    """Arm `arm_b` of `exp_b` against arm `arm_a` of `exp_a` over their
    cells (cells_of: `cells` names some), labelled EXP:ARM."""
    a, b = sides(exp_a, [arm_a])[arm_a], sides(exp_b, [arm_b])[arm_b]
    return scorecard(cells_of(exp_a, exp_b, cells), a, b, f"{exp_a}:{arm_a}", f"{exp_b}:{arm_b}", show_all, traces)


def arm_scores(exp, base, arms=None, cells=None, rank="lag", lag_budget=None, lag_ceiling=None):
    """Each of `arms` of `exp` (its every arm but `base` by default: the
    experiment's arm_labels) against `base` over its cells (`cells` names
    some), titled EXP:ARM against EXP:BASE, best first (rank_key)."""
    if arms is None:
        labels = query(f"SELECT arm_labels FROM experiments WHERE exp_id = {q(exp)};")
        arms = [x for x in (labels[0]["arm_labels"] if labels else []) if x != base]
    got, where = sides(exp, [base, *arms]), cells_of(exp, only=cells)
    out = [arm_score(where, got[base], got[arm], f"{exp}:{arm} against {exp}:{base}", rank, lag_budget, lag_ceiling)
           for arm in arms]
    return sorted(out, key=lambda sc: sc.rank_key)


def structural(exp):
    """The cells where every arm of `exp` collapsed at every seed it ran
    (structural): {cell, arms, runs}."""
    return query(f"SELECT * FROM structural({q(exp)});")


def dynamics(exp):
    """Each arm and cell of `exp` in the terms that tell a startup tail from
    a steady spike (dynamics: scoring.sql says what each column is)."""
    return query(f"SELECT * FROM dynamics({q(exp)});")
