# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""Seed coverage: how well the seed's sends cover every stream, the
measure that judges the work on the HAVE removal's seed. A descriptive
report over the catalog's truth tables, read through simlab.engine: the
holds (`arrivals`: every node's first hold of every chunk, its route and
source), the push trees (`parents`: every node's push parent of every
stream, as it changed), the viewers' phases (`viewer_phases`) and the
run's (`verdict_seconds`).

  Selection      simlab.forecast's: experiments, cells and arm labels
                 (globs), seeds, arm names by label glob (`names`), the
                 phases kept (`phases`, of a chunk's publication or a
                 stream-second: REST, 'all' or a list) and the runs by
                 their verdict (`collapsed`)
  report(sel, sections=SECTIONS, conn=None, say=None, warn=None)
                 -> {section: Result}
  report_text(results) -> str
  prepare(conn, sel) -> [warning]
                 the measure's tables on a connection (runs_sql,
                 tables_sql)

hack/sim-db/simdb.py coverage-report prints the report.

The measure's definitions are the catalog's macros (hack/simlab/sql/
scoring.sql), one definition for this report and any view of the
console's, which this module's sections group: coverage_runs_of(jobs) a
run's facts, and over a list of its rows seed_children_in(runs) the
seed's child subscriptions, coverage_chunks_in(runs) a chunk's measure (a
late one's class among it) and seed_cover_in(runs) I1's stream-seconds
(each `_in` with its `_of(jobs)` form). prepare loads this checkout's
macros into the session (scoring.load), whatever the catalog's indexer
had. A chunk at a time:

  seed       the seed series_meta records (`seed`), else the node that
             published the most (arrivals' `own` rows)
  pub_ms     the seed's own-store `arrivals` row (how `own`)
  stream     seq mod the run's stream count: the one the sim records
             (series_meta's `streams`, else the ppspp sim's row's), else
             inferred (one more than the greatest stream its `parents`
             name, else its `asks`', else one); the report warns of
             every run whose count it inferred, and the latency section
             counts them (inferred_streams)
  interval   the run's chunk interval: the median of every gap between
             consecutive publications within a stream (the signing
             cadence: 250 ms in the suite cells), taken from the data
  first      the earliest `arrivals` row of any node but the seed (ties
             by node), any incarnation: first_ms, its route (`push` or
             `answer` as arrivals' how names them, anything else
             `other`) and its source (arrivals' peer: `seed`, else
             `peer` or `unnamed` when no source is named: both an
             anomaly, a first copy the seed didn't send; NULL for a
             chunk never held)
  latency    first_ms - pub_ms
  never      no node but the seed held it by the run's end
  end        the run's end: the end of its last second in `seconds`
             (strims-sim runs on 10 s past its stream's end, and holds
             come in then), or its last hold if that is later
  judged     published at least the latency budget's most
             (verdict.DELIVERY_MAX_MS) before the run's end; every other
             chunk, held or not, is censored and counted apart: whether
             it would come within the budget is unknown, and leaving out
             only the never-held ones of them would bias the late share
             down
  late       a judged chunk held later than its run's interval after its
             publication, or never
  phase      the run's phase of its publication second (verdict_seconds,
             the run's own windows: startup is the run's first 25 s, its
             seconds 0-24, so a chunk published at exactly 25 s is
             steady); a chunk is the swarm's, so a viewer's own startup
             after its join isn't the chunk's. A publication past the
             verdict's seconds is the vocabulary's tail (vocab.TAIL)

Quantiles of latency are of the held chunks (sketch_quantile): never is a
route of its own, apart, beside them, and the late share counts it.

The 25 s edge is the phases': a chunk published at 25.000 s is in second
25, the first steady one, so a probe taking pub_ms > 25000 leaves out
the chunks a run publishes then (one a stream). A job counts once in
each cell and arm the selected experiments label it with
(forecast.jobs_sql). The seeds are the selection's, pooled within a
group with each run's spread beside: a cell some experiments ran at more
seeds than the others (2004 and 3033 beside 1-8) pools more runs unless
`seed` picks them. A never-held chunk is never unless censored; past the
suite cells' streams strims-sim runs another 10 s, so its chunks are
seldom censored, while the ppspp sim's runs end about a second after
their last publication.

A run whose arrivals the catalog lacks (it wrote none, or they weren't
fetched) has no chunks: it is left out of every section, with a warning,
and a selection of no run with arrivals is refused (query.need). A run
without parents rows (no push trees: the ppspp sim without hybrid push)
has no seed children to count: its I1 and its late chunks' classes are
NULL, it is left out of the late section, and the report warns of it.
The warnings go to `warn`: simdb.py prints them on stderr, and its
`--json` carries them as `warnings`.

The sections (SECTIONS), each grouped by arm, cell, collapsed (the run's
verdict: collapsed runs apart) and phase, a row each phase kept, and a
row of phase NULL pooling every kept phase past startup (the brief's
"outside 25 s") where a group has two or more of those, with absolute
counts beside shares and each run's spread (the least, median and
greatest over the group's runs, `*_runs`):

  latency    the judged chunks: how many, how many censored, held, never
             and late (each a count and a share), latency p50/p90/p99
             and max of the held, anomalies, the runs whose stream count
             was inferred (inferred_streams), and the runs' intervals
             (interval_runs)
  routes     each first copy's route and source: chunks, share, late,
             latency p50/p99 and max
  invariants I1, the invariant that every stream a viewer watches has a
             seed child: stream-seconds with no seed child while an honest
             viewer watches (a viewer_phases second of any honest viewer;
             every viewer wants every stream), from `parents`: a node is
             the seed's child of a stream from a row naming the seed its
             parent to the node's next row of the stream, or the run's
             end. Watched stream-seconds of the runs with parents rows,
             the time in them with no seed child (uncovered_s, ms exact),
             its share, the stream-seconds with any of it, the mean seed
             children of a stream (time weighted), the runs without
             parents rows (unparented), and how the seed's child
             subscriptions ended in the phase (ends_<reason>, the next
             row's reason: the vocabulary's parents reasons)
  late       each late chunk of a run with parents rows, classed by the
             seed's children of its stream at publication (first match):
             no_seed_child (with `after`, how the stream's last seed
             child before it ended, a parents reason, NULL for a stream
             that never had one), released (a child's hold names a
             release to pull, arrivals' released_ms: push's deadline or
             the parent's going gave up the seed's push; a CANCEL with it
             on the HAVE removal), pushed_late (a child holds it by the
             seed's push), child_never (no child at publication ever held
             it), other (a child holds it otherwise, by an answer without
             a release); chunks, their share of the late and of the
             judged, the children at publication, latency p50 of the held

Rows of `parents` at one ms of one node and stream are ordered as they
can have happened (seed_children_in): a `switch` first, an `open` after
an end if the node had a parent before that ms (an open needs none) and
before it if not, the ends between.
"""

import sys

from . import engine, forecast, vocab
from .forecast import REST, Selection, connected  # noqa: F401
from .store_host import q

SECTIONS = ("latency", "routes", "invariants", "late")
# The late classes, coverage_chunks_in's, in the order a report gives them.
LATE_CLASSES = ("no_seed_child", "released", "pushed_late", "child_never", "other")
# How a seed child's subscription ended (parents' reasons): the
# invariants' ends_<reason> columns.
ENDS = tuple(vocab.reasons())
STARTUP = vocab.named("phases", "startup")
# The phases' order in a report, the pooled row (phase NULL) last.
PHASE_ORDER = tuple(vocab.phases())
# Where coverage_runs_of's stream count was read from (the others, it was
# inferred).
RECORDED = ("series_meta", "row")
# The pooled group's phase within a section's SQL: NULL in its rows.
POOLED = "*"


def phase_groups(col, row, pools="cv_pools"):
    """SQL: the phase groups of a row of phase `col` and run labels
    `row`.arm/cell/collapsed: its own and, but for startup, the pooled one
    (POOLED) where its group has two phases or more past startup
    (`pools`' rows: pools_sql's)."""
    pooled = (f"EXISTS (SELECT 1 FROM {pools} p WHERE p.arm = {row}.arm AND p.cell = {row}.cell "
              f"AND p.collapsed = {row}.collapsed)")
    return (f"unnest(CASE WHEN {col} <> {q(STARTUP)} AND {pooled} THEN [{col}, {q(POOLED)}] "
            f"ELSE [{col}] END)")


def phase_rank(col="phase"):
    whens = " ".join(f"WHEN {q(p)} THEN {i}" for i, p in enumerate(PHASE_ORDER))
    return f"CASE {col} {whens} ELSE {len(PHASE_ORDER)} END"


def spread(col):
    """SQL: the least, median (sketch_quantile's) and greatest of `col` over a group's runs, NULL
    for none."""
    return f"CASE WHEN count({col}) > 0 THEN [min({col}), sketch_quantile({col}, 50), max({col})] END"


def runs_sql(sel):
    """The statements making cv_sel (the jobs, a row a job and label:
    forecast.jobs_sql) and cv_runs (their facts, a row a job:
    coverage_runs_of)."""
    return f"""{forecast.jobs_sql(sel, "cv_sel")}
CREATE OR REPLACE TEMP TABLE cv_runs AS FROM coverage_runs_of((SELECT list({{'k': k}}) FROM cv_sel));
"""


def tables_sql(sel):
    """The statements making the measure's other tables over the runs
    cv_sel and cv_runs keep, each from scoring.sql's macro of its
    definition:

      cv_jobs     cv_sel's columns and cv_runs', a row a job and label
      cv_chunks   a row a published chunk and label (coverage_chunks_in),
                  with the label (arm, cell, seed, sim, collapsed), the
                  run's streams_from and parented (it has parents rows)
      cv_subs     the seed's child subscriptions (seed_children_in), a
                  job's own
      cv_late     a row a judged late chunk of a run with parents rows,
                  in every phase: k, src, seq, stream, pub_ms and its
                  class (cv_chunks' children, class, after), a job's own
      cv_cover    I1's watched stream-seconds (seed_cover_in), a row each
                  label
      cv_phase    k, src, sec, phase: the run's (verdict_seconds)
      cv_pools    the groups whose chunks report a pooled row (pools_sql)
    """
    runs = "(SELECT list(r) FROM cv_runs r)"
    labels = "j.arm, j.cell, j.seed, j.sim, j.collapsed"
    return f"""
CREATE OR REPLACE TEMP TABLE cv_jobs AS
SELECT j.*, r.* EXCLUDE (k, src) FROM cv_sel j JOIN cv_runs r USING (k, src);
CREATE OR REPLACE TEMP TABLE cv_chunks AS
SELECT {labels}, r.streams_from, r.parents_rows > 0 AS parented, c.* FROM coverage_chunks_in({runs}) c
JOIN cv_sel j USING (k, src) JOIN cv_runs r USING (k, src);
CREATE OR REPLACE TEMP TABLE cv_subs AS FROM seed_children_in({runs});
CREATE OR REPLACE TEMP TABLE cv_late AS
SELECT DISTINCT k, src, seq, stream, pub_ms, children, class, after FROM cv_chunks WHERE class IS NOT NULL;
CREATE OR REPLACE TEMP TABLE cv_cover AS
SELECT {labels}, c.* FROM seed_cover_in({runs}) c JOIN cv_sel j USING (k, src);
CREATE OR REPLACE TEMP TABLE cv_phase AS
SELECT v.k, v.src, v.sec, v.phase FROM verdict_seconds v SEMI JOIN cv_sel USING (k, src);
CREATE OR REPLACE TEMP TABLE cv_pools AS {pools_sql(sel)};
"""


def pools_sql(sel, rows="cv_chunks"):
    """SQL: the arm, cell and verdict groups of `rows` (arm, cell,
    collapsed, phase) whose kept phases past startup are two or more: the
    groups a section reports a pooled row for (cv_pools: the chunks')."""
    return (f"SELECT arm, cell, collapsed FROM {rows} WHERE phase <> {q(STARTUP)} AND {forecast.phase_filter(sel)} "
            "GROUP BY ALL HAVING count(DISTINCT phase) > 1")


def prepare(conn, sel):
    """Makes the measure's tables for `sel` on `conn` (runs_sql, then
    tables_sql), with this checkout's scoring macros in its session
    (scoring.load), and returns the report's warnings. A run without
    arrivals has no chunks: it is left out of every section, and the
    selection refused (query.Refused) when no run of it has them."""
    from . import query, scoring

    scoring.load(conn)
    for s in conn.extract_statements(runs_sql(sel)):
        conn.execute(s)
    said = []
    bare = conn.execute("SELECT count(*) FILTER (WHERE wrote), count(*) FILTER (WHERE NOT wrote) FROM ("
                        "SELECT r.k, bool_or(coalesce(list_contains(string_split(w.wrote, ','), 'arrivals'), false)) "
                        "AS wrote FROM cv_runs r LEFT JOIN raw.runs w USING (k, src) WHERE r.arrivals_rows = 0 "
                        "GROUP BY r.k)").fetchone()
    if bare[0]:
        said.append(f"coverage: {bare[0]} runs' arrivals aren't in the catalog (simdb.py fetch --tables arrivals "
                    "brings them from S3): left out of every section")
    if bare[1]:
        said.append(f"coverage: {bare[1]} runs wrote no arrivals ({query.knobs_text('arrivals')} writes them): "
                    "left out of every section")
    if sum(bare):
        had = conn.execute("SELECT count(*) FROM cv_runs").fetchone()[0]
        conn.execute("DELETE FROM cv_sel WHERE k IN (SELECT k FROM cv_runs WHERE arrivals_rows = 0)")
        conn.execute("DELETE FROM cv_runs WHERE arrivals_rows = 0")
        if had == sum(bare):
            query.need(set(), "arrivals")
    for s in conn.extract_statements(tables_sql(sel)):
        conn.execute(s)
    unparented = conn.execute("SELECT count(*) FROM cv_runs WHERE parents_rows = 0").fetchone()[0]
    if unparented:
        said.append(f"coverage: {unparented} runs have no parents rows (no push trees): their I1 and their late "
                    "chunks' classes are NULL")
    got = conn.execute(f"SELECT streams_from, count(*) FROM cv_runs WHERE streams_from IS NULL OR streams_from NOT IN "
                       f"({', '.join(q(x) for x in RECORDED)}) GROUP BY ALL ORDER BY streams_from NULLS LAST"
                       ).fetchall()
    if got:
        how = ", ".join(f"{n} from {what}" if what else f"{n} with one assumed" for what, n in got)
        said.append(f"coverage: {sum(n for _, n in got)} runs' stream count is inferred, not read ({how}): neither "
                    "their series_meta nor their row records it, and their interval and I1 take the count inferred")
    return said


def grouped(sel):
    """SQL: cv_chunks' rows `sel` keeps by their phase, each in its phase
    groups."""
    return (f"(SELECT c.* EXCLUDE (phase), {phase_groups('c.phase', 'c')} AS phase FROM cv_chunks c "
            f"WHERE {forecast.phase_filter(sel, 'c.phase')})")


GROUP = "arm, cell, collapsed, phase"
# A section's group as its rows give it: the pooled one's phase NULL.
SHOWN = f"arm, cell, collapsed, nullif(phase, {q(POOLED)}) AS phase"


def latency_sql(sel):
    return f"""
WITH g AS {grouped(sel)},
per AS (
    SELECT {GROUP}, k, any_value(interval_ms) AS interval_ms, avg(late::INT) AS late,
           avg((route = 'never')::INT) AS never, sketch_quantile(latency_ms, 99) AS p99
    FROM g WHERE judged GROUP BY ALL
), s AS (
    SELECT {GROUP}, {spread('interval_ms')} AS interval_runs, {spread('late')} AS late_runs,
           {spread('never')} AS never_runs, {spread('p99')} AS p99_runs
    FROM per GROUP BY ALL
)
SELECT {SHOWN}, count(DISTINCT g.k) AS runs, list(DISTINCT g.seed ORDER BY g.seed) AS seeds,
       count(*) FILTER (WHERE judged) AS chunks, count(*) FILTER (WHERE NOT judged) AS censored,
       count(first_ms) FILTER (WHERE judged) AS held,
       count(*) FILTER (WHERE judged AND route = 'never') AS never,
       avg((route = 'never')::INT) FILTER (WHERE judged) AS never_share,
       count(*) FILTER (WHERE judged AND late) AS late, avg(late::INT) FILTER (WHERE judged) AS late_share,
       sketch_quantile(CASE WHEN judged THEN latency_ms END, 50) AS p50_ms,
       sketch_quantile(CASE WHEN judged THEN latency_ms END, 90) AS p90_ms,
       sketch_quantile(CASE WHEN judged THEN latency_ms END, 99) AS p99_ms,
       max(latency_ms) FILTER (WHERE judged) AS max_ms,
       count(*) FILTER (WHERE judged AND source IN ('peer', 'unnamed')) AS anomalies,
       count(DISTINCT g.k) FILTER (WHERE g.streams_from IS NULL
                                   OR g.streams_from NOT IN ({', '.join(q(x) for x in RECORDED)})) AS inferred_streams,
       any_value(s.interval_runs) AS interval_runs, any_value(s.late_runs) AS late_runs,
       any_value(s.never_runs) AS never_runs, any_value(s.p99_runs) AS p99_runs
FROM g LEFT JOIN s USING (arm, cell, collapsed, phase)
GROUP BY {GROUP} ORDER BY arm, cell, collapsed, {phase_rank()};
"""


def routes_sql(sel):
    return f"""
WITH g AS (SELECT * FROM {grouped(sel)} WHERE judged),
tot AS (SELECT {GROUP}, k, count(*) AS n FROM g GROUP BY ALL),
per AS (SELECT {GROUP}, route, source, k, count(*) AS n FROM g GROUP BY ALL),
combos AS (SELECT DISTINCT {GROUP}, route, source FROM per),
shares AS (
    SELECT c.*, t.k, coalesce(p.n, 0) / t.n AS share
    FROM combos c JOIN tot t USING (arm, cell, collapsed, phase)
    LEFT JOIN per p ON p.arm = c.arm AND p.cell = c.cell AND p.collapsed = c.collapsed AND p.phase = c.phase
         AND p.route = c.route AND p.source IS NOT DISTINCT FROM c.source AND p.k = t.k
), s AS (SELECT {GROUP}, route, source, {spread('share')} AS share_runs FROM shares GROUP BY ALL),
all_n AS (SELECT {GROUP}, sum(n) AS n FROM tot GROUP BY ALL)
SELECT g.arm, g.cell, g.collapsed, nullif(g.phase, {q(POOLED)}) AS phase, g.route, g.source, count(*) AS chunks,
       count(*) / any_value(a.n) AS share, count(*) FILTER (WHERE g.late) AS late,
       sketch_quantile(g.latency_ms, 50) AS p50_ms, sketch_quantile(g.latency_ms, 99) AS p99_ms,
       max(g.latency_ms) AS max_ms, any_value(s.share_runs) AS share_runs
FROM g JOIN all_n a ON a.arm = g.arm AND a.cell = g.cell AND a.collapsed = g.collapsed AND a.phase = g.phase
JOIN s ON s.arm = g.arm AND s.cell = g.cell AND s.collapsed = g.collapsed AND s.phase = g.phase
     AND s.route = g.route AND s.source IS NOT DISTINCT FROM g.source
GROUP BY g.arm, g.cell, g.collapsed, g.phase, g.route, g.source
ORDER BY g.arm, g.cell, g.collapsed, {phase_rank('g.phase')}, chunks DESC, g.route, g.source;
"""


def invariants_sql(sel):
    ends = ", ".join(f"count(*) FILTER (WHERE end_reason = {q(e)}) AS ends_{e}" for e in ENDS)
    return f"""
WITH pools AS ({pools_sql(sel, "cv_cover")}
), x AS (
    SELECT s.* EXCLUDE (phase), {phase_groups('s.phase', 's', 'pools')} AS phase
    FROM cv_cover s WHERE {forecast.phase_filter(sel, 's.phase')}
), per AS (
    SELECT {GROUP}, k, 1 - sum(covered_ms) / nullif(count(covered_ms) * 1000, 0) AS uncovered FROM x GROUP BY ALL
), s AS (SELECT {GROUP}, {spread('uncovered')} AS uncovered_runs FROM per GROUP BY ALL),
e AS (
    SELECT {GROUP}, {ends} FROM (
        SELECT j.arm, j.cell, j.collapsed, u.end_reason, {phase_groups('ph.phase', 'j', 'pools')} AS phase
        FROM cv_subs u JOIN cv_jobs j USING (k, src)
        JOIN cv_phase ph ON ph.k = u.k AND ph.src = u.src AND ph.sec = u.close_ms // 1000
        WHERE u.end_reason IS NOT NULL AND {forecast.phase_filter(sel, 'ph.phase')}
    ) GROUP BY ALL
)
SELECT {SHOWN}, count(DISTINCT x.k) AS runs, list(DISTINCT x.seed ORDER BY x.seed) AS seeds,
       count(DISTINCT x.k) FILTER (WHERE covered_ms IS NULL) AS unparented,
       count(covered_ms) AS stream_s, (count(covered_ms) * 1000 - sum(covered_ms)) / 1000 AS uncovered_s,
       1 - sum(covered_ms) / nullif(count(covered_ms) * 1000, 0) AS uncovered,
       count(*) FILTER (WHERE covered_ms < 1000) AS touched_s,
       sum(child_ms) / nullif(count(covered_ms) * 1000, 0) AS seed_children,
       any_value(s.uncovered_runs) AS uncovered_runs,
       {", ".join(f"coalesce(any_value(e.ends_{x}), 0) AS ends_{x}" for x in ENDS)}
FROM x LEFT JOIN s USING (arm, cell, collapsed, phase) LEFT JOIN e USING (arm, cell, collapsed, phase)
GROUP BY {GROUP} ORDER BY arm, cell, collapsed, {phase_rank()};
"""


def late_sql(sel):
    return f"""
WITH g AS (SELECT * FROM {grouped(sel)} WHERE judged AND parented),
l AS (SELECT * FROM g WHERE class IS NOT NULL),
tot AS (SELECT {GROUP}, k, count(*) AS n, count(*) FILTER (WHERE late) AS late_n FROM g GROUP BY ALL),
all_n AS (SELECT {GROUP}, sum(n) AS n, sum(late_n) AS late_n FROM tot GROUP BY ALL),
per AS (SELECT {GROUP}, class, after, k, count(*) AS n FROM l GROUP BY ALL),
combos AS (SELECT DISTINCT {GROUP}, class, after FROM per),
shares AS (
    SELECT c.*, t.k, coalesce(p.n, 0) / t.n AS share
    FROM combos c JOIN tot t USING (arm, cell, collapsed, phase)
    LEFT JOIN per p ON p.arm = c.arm AND p.cell = c.cell AND p.collapsed = c.collapsed AND p.phase = c.phase
         AND p.class = c.class AND p.after IS NOT DISTINCT FROM c.after AND p.k = t.k
), s AS (SELECT {GROUP}, class, after, {spread('share')} AS share_runs FROM shares GROUP BY ALL)
SELECT l.arm, l.cell, l.collapsed, nullif(l.phase, {q(POOLED)}) AS phase, l.class, l.after, count(*) AS chunks,
       count(*) / any_value(a.late_n) AS late_share, count(*) / any_value(a.n) AS share,
       avg(l.children) AS children, count(*) FILTER (WHERE l.route = 'never') AS never,
       sketch_quantile(l.latency_ms, 50) AS p50_ms, any_value(s.share_runs) AS share_runs
FROM l JOIN all_n a ON a.arm = l.arm AND a.cell = l.cell AND a.collapsed = l.collapsed AND a.phase = l.phase
JOIN s ON s.arm = l.arm AND s.cell = l.cell AND s.collapsed = l.collapsed AND s.phase = l.phase
     AND s.class = l.class AND s.after IS NOT DISTINCT FROM l.after
GROUP BY l.arm, l.cell, l.collapsed, l.phase, l.class, l.after
ORDER BY l.arm, l.cell, l.collapsed, {phase_rank('l.phase')}, list_position({list(LATE_CLASSES)!r}, l.class),
         chunks DESC, l.after;
"""


SECTION_SQL = {"latency": latency_sql, "routes": routes_sql, "invariants": invariants_sql, "late": late_sql}


def report(sel, sections=SECTIONS, conn=None, say=None, warn=None):
    """Each section's Result over `sel`'s jobs (module docs), in order;
    each of prepare's warnings to `warn` (a list's append, say: the
    command's --json carries them), by default stderr."""
    unknown = [s for s in sections if s not in SECTION_SQL]
    if unknown:
        raise ValueError(f"no section {', '.join(map(repr, unknown))}: the sections are {', '.join(SECTIONS)}")
    out = {}
    with connected(conn) as c:
        for w in prepare(c, sel):
            (warn or (lambda m: print(m, file=sys.stderr)))(w)
        for s in sections:
            if say:
                say(s)
            out[s] = engine.run(c, SECTION_SQL[s](sel))
    return out


def report_text(results):
    """The report's text: each section's table."""
    return forecast.report_text(results)
