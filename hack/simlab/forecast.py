# Copyright 2026 Strims contributors
# SPDX-License-Identifier: AGPL-3.0-only
"""The arrival forecast study: is time the right predictor of when a
neighbour holds a chunk? A descriptive report over the catalog's decision
logs, read through simlab.engine: the HAVE removal's `asks` (every request
decision, the asked peer's row and every rival's, with the arrival model's
prediction) against the truth the sims know (`arrivals`: every node's
first hold of every chunk), and main's `haves` and `asks` (the HAVE world's
knowledge delay, and its requests on that knowledge).

  Selection(exp, cell, arm, seed, names, phases, collapsed)
                 the jobs a report reads: experiments, cells and arm labels
                 (globs), seeds; `names` an arm's name by label glob, so
                 one arm's two sims' labels (a ppspp sim's and strims-sim's)
                 report as one; `phases` the requester's viewer phases kept
                 ('rest', every one but startup, by default); `collapsed`
                 the runs kept by their verdict (every one by default)
  report(sel, sections=SECTIONS, conn=None, say=None) -> {section: Result}
  report_text(results) -> str
  prepare(conn, sel), connected(conn=None)
                 the study's tables on a connection (prepare_sql), and a
                 reader of the catalog for them; simlab.regimes builds on
                 both
  jobs_sql(sel, table)
                 the statement making a selection's jobs, a row a job and
                 label, with their verdicts
  explained_sql(explainers, source, cols), state_explained_sql(),
  agreement_sql()
                 explained's statement for other classes (regimes' decoded
                 states), and how much requesters agree beside peers

hack/sim-db/simdb.py forecast-report prints the report (and regimes').

A row of the study is a decision's (receiver, peer, chunk): an `asks` row
of an honest requester and honest peer whose truth is live (the peer's
hold came after the channel opened and the requester's incarnation began)
or which the peer never held. The asked rows (`picked`) carry the
outcomes; every row carries a prediction where the model had one. A row
is kept when the requester's phase at the decision is one the selection
keeps; every section reads the kept rows alone but `aligned`, whose
windows open in any phase.

Each row is classed by the structure the sims know of it at the decision,
which the arrival model doesn't read (STRUCTURE):

  path      how the peer came to hold the chunk (arrivals' `how`): own
            (the seed, publishing it), push (its push parent sent it: the
            peer was on the chunk's push path), answer (it pulled it:
            off the path), other, or never
  supply    unpublished (the seed hadn't published it by the decision),
            seed_only (published, no other node held it yet), spread
  position  the chunk against the peer's latest STATE: no_state, past
            (after its frontier), hole (its declared newest hole), behind
            (before that hole: declared, as a summary reads it, but
            unsaid), declared (between the hole and the frontier)
  elsewhere declared_elsewhere where a rival the decision weighed, not
            the seed, declared the chunk between its hole and frontier,
            else not_elsewhere: what the requester itself could know of
            the chunk's supply (with position, the structure it observes;
            path and supply are the sims' truth)

The arrival model's predictive distribution is its three logged quantiles
(p50, p90, p99). Its CDF, where a score needs one, runs linearly through
(L, 0), (p50, 0.5), (p90, 0.9), (p99, 0.99) and stays at 0.99 past p99, L
the earlier of the decision and p50: the model conditions a chunk it
lacks on not having come by now, and the 1 % past p99 is the mass it
leaves unsaid ("never by this path"). The PIT histogram needs no such
interpolation: its bins are the quantiles' own (held by p50, by p90, by
p99, later), expected 50/40/9/1 %, a hold at a quantile counting by it as
the asks view's held_by_* do, and a hold never seen counts past p99 (a
censored outcome whose censoring time, the run's end, lies past every
prediction: D-calibration's rule puts it there).

The sections (SECTIONS):

  runs       the runs by arm, cell and verdict (collapsed or not): their
             seeds, the kept asks a run made (least, median, most), and
             their failure rate and shares of the flight round, of
             seed-only chunks and of chunks at or behind the peer's
             declared hole: a run that collapses asks in storms, and
             pooled numbers are mostly its
  decompose  the premise test's first half: for each factor (STRUCTURE)
             and level, the asked rows' share, failure rate
             (outcome not_yet or expired), share of all failures, and the
             prediction error (held - p50) of the rows with a hold, with
             the factor's share of the error's variance (eta squared,
             errors clipped at +-CLIP_MS)
  explained  the premise test's second half: how well each predictor of a
             request's failure scores, by log loss and Brier score, and
             its skill against the base rate (McFadden's R squared): the
             model (1 - F(arrival + grace)), the model recalibrated (the
             failure rate of its decile of that probability), each
             structural factor, the observable class (position and
             elsewhere) and the joint class (path, supply, position), each
             of those two also crossed with the model's quintile (failure
             rates). Rates are fitted leaving the row's seed out, so a
             class's rate is another seeds'
  scores     proper scores of the model's quantiles by round and the
             requester's phase: pinball loss at each quantile, the
             quantile form of WIS (the mean of twice the pinball losses),
             coverage at 50/90/99 (a never-held chunk uncovered), the PIT
             histogram, sharpness (the median p90 - p50 and p99 - p50, and
             the share without spread). A never-held row's pinball loss is
             its lower bound, the loss at its censoring time
  aligned    the same scores aligned on events, in ALIGN_BIN_S bins from
             -ALIGN_BEFORE_S to ALIGN_AFTER_S: the run's first scenario
             event of each kind (ALIGN_KINDS), the asked peer's own parent
             changes of the chunk's stream (kind parent_change, each row
             on the latest that came by ALIGN_BEFORE_S after it), and the
             requester's and the asked peer's arrival into the running
             swarm (receiver_join, peer_join)
  baselines  the model against two naive forecasters of the same rows,
             each a point plus quantiles of its errors in the other seeds
             of its arm, cell and round (the median over those seeds of
             each seed's quantile): persistence (the chunk's publication
             plus the lag the peer's declared frontier had: its hold less
             its publication) and cadence (the frontier's hold plus the
             stream's publishing interval per chunk past it); skill is 1 -
             WIS(model) / WIS(baseline) on rows with a hold
  staleness  scores against the asked peer's STATE age at the decision
             (STATE_AGES_MS) and against the time since the peer's last
             parent change of the stream (BREAK_AGES_S)
  selection  the picked peer's prediction error against its rivals' in
             the same decision (the winner's curse), how often the picked
             peer missed (no hold by its arrival and grace) while a rival
             held the chunk by its own arrival (the decision's regret),
             apart for a peer that held it and for the seed alone, and how
             often the seed was the one asked
  bound      main as the upper bound: the HAVE knowledge delay (a HAVE's
             arrival less the announcing peer's hold), and the outcomes
             of main's requests on that knowledge beside decl's and
             p4tf's, by cell: each outcome's share, the share failed
             (FAILED) and its least and greatest over the seeds

Every section is grouped by arm and cell. Runs carry structure that a
pooled number hides, so each section keeps its groups apart. A job counts
once in each cell and arm the selected experiments label it with
(jobs_sql): a farm job reused in another cell counts in both, and one
two experiments share under one label counts once.
"""

import contextlib
import dataclasses

from . import engine, vocab
from .store_host import q

SECTIONS = ("runs", "decompose", "explained", "scores", "aligned", "baselines", "staleness", "selection", "bound")
# The structural factors, each a column of the study's rows.
STRUCTURE = ("path", "supply", "position", "elsewhere")
# Errors past this are clipped for the variance shares: a never-settled
# chunk's error would otherwise be the whole variance.
CLIP_MS = 5000
# The logged quantiles, their levels and the PIT bins' expected shares.
QUANTILES = (("p50", 0.5), ("p90", 0.9), ("p99", 0.99))
PIT_EXPECTED = (0.5, 0.4, 0.09, 0.01)
# Probabilities clipped for the log loss.
EPS = 0.001
# Event alignment: bins, the window, and the run-level kinds (each run's
# first after its start).
ALIGN_BIN_S = 2
ALIGN_BEFORE_S = 10
ALIGN_AFTER_S = 30
ALIGN_KINDS = ("uplink_rate", "depart", "crash", "rate_limit", "latency")
# Staleness buckets' lower edges.
STATE_AGES_MS = (0, 50, 100, 250, 500, 1000)
BREAK_AGES_S = (0, 1, 3, 10, 30)
# The requester's phases a report keeps by default: every one but startup.
REST = "rest"
STARTUP = vocab.named("phases", "startup")


@dataclasses.dataclass
class Selection:
    """The jobs a report reads (`exp`, `cell`, `arm` globs over
    experiment_jobs' exp_id, cell and arm_label; `seed` a list), the arm
    names by label glob (`names`: {glob: name}, first match; else the
    label), the requester's phases kept (`phases`: REST, 'all', or a
    list of phases), and the runs by their verdict (`collapsed`: None for
    every run, else those that collapsed or those that didn't)."""
    exp: list = None
    cell: list = None
    arm: list = None
    seed: list = None
    names: dict = None
    phases: object = REST
    collapsed: bool = None


def globs(col, pats):
    """SQL: `col` matching any of the globs `pats` (true without any)."""
    if not pats:
        return "true"
    return "(" + " OR ".join(f"{col} GLOB {q(p)}" for p in pats) + ")"


def arm_name(sel):
    """SQL: an experiment job's arm name, by `sel.names`."""
    if not sel.names:
        return "arm_label"
    whens = " ".join(f"WHEN arm_label GLOB {q(g)} THEN {q(n)}" for g, n in sel.names.items())
    return f"CASE {whens} ELSE arm_label END"


def phase_filter(sel, col="phase"):
    """SQL: the study's rows `sel.phases` keeps."""
    if sel.phases in (None, "all"):
        return "true"
    if sel.phases == REST:
        return f"coalesce({col}, '') <> {q(STARTUP)}"
    return f"{col} IN ({', '.join(q(p) for p in sel.phases)})"


def interp(x, row="a"):
    """SQL: the model's CDF at time `x` (module docs), from row `row`'s
    t_ms and quantiles."""
    lo = f"least({row}.t_ms, {row}.p50)"
    return (f"(CASE WHEN {x} >= {row}.p99 THEN 0.99 "
            f"WHEN {x} >= {row}.p90 THEN 0.9 + 0.09 * ({x} - {row}.p90) / nullif({row}.p99 - {row}.p90, 0) "
            f"WHEN {x} >= {row}.p50 THEN 0.5 + 0.4 * ({x} - {row}.p50) / nullif({row}.p90 - {row}.p50, 0) "
            f"WHEN {x} >= {lo} THEN 0.5 * ({x} - {lo}) / nullif({row}.p50 - {lo}, 0) "
            "ELSE 0.0 END)")


def cdf(x, row="a"):
    """SQL: interp's CDF, a quantile's level where two quantiles meet."""
    return (f"coalesce({interp(x, row)}, CASE WHEN {x} >= {row}.p99 THEN 0.99 WHEN {x} >= {row}.p90 THEN 0.9 "
            f"WHEN {x} >= {row}.p50 THEN 0.5 ELSE 0.0 END)")


def pinball(y, qcol, tau):
    """SQL: the pinball loss in seconds of quantile `qcol` at level `tau`
    for outcome `y` (ms)."""
    return f"(({y}) - ({qcol})) * ({tau} - (({y}) < ({qcol}))::INT) / 1000.0"


def position_sql(row):
    """SQL: row `row`'s chunk against its peer's latest STATE (module
    docs' position)."""
    r = row
    return (f"CASE WHEN {r}.frontier_seq IS NULL THEN 'no_state' WHEN {r}.seq > {r}.frontier_seq THEN 'past' "
            f"WHEN {r}.seq = {r}.hole_seq THEN 'hole' WHEN {r}.seq < {r}.hole_seq THEN 'behind' ELSE 'declared' END")


def jobs_sql(sel, table):
    """The statement making temp table `table` of `sel`'s jobs, a row a job
    and label: k, src, arm (by `sel.names`), cell, seed, sim and collapsed
    (its verdict's, false without one), those `sel.collapsed` keeps. A job
    counts once in each cell and arm the selected experiments label it
    with: a job two experiments share under one label is one row, and one
    they label apart (a farm job reused in another cell, an arm a --sets
    run names otherwise) a row each."""
    seeds = f"seed IN ({', '.join(str(int(s)) for s in sel.seed)})" if sel.seed else "true"
    collapsed = "true" if sel.collapsed is None else ("collapsed" if sel.collapsed else "NOT collapsed")
    where = " AND ".join([globs("exp_id", sel.exp), globs("cell", sel.cell), globs("arm_label", sel.arm), seeds])
    return f"""
CREATE OR REPLACE TEMP TABLE {table} AS
SELECT DISTINCT * FROM (
    SELECT e.k, e.src, {arm_name(sel)} AS arm, e.cell, e.seed, e.sim, coalesce(v.collapsed, false) AS collapsed
    FROM exp_runs e LEFT JOIN verdicts v USING (k, src) WHERE {where}
) WHERE {collapsed};"""


def prepare_sql(sel):
    """The statements making the study's tables for `sel`:

      fs_sel     the jobs, a row a job and label (jobs_sql); the tables
                 of a job's own (fs_arr, fs_seed, fs_pub, fs_rate,
                 fs_holds, fs_phase) are a row a job's, and those of the
                 decisions (fs_all, fs_asks) a row each of its labels
      fs_jobs    k, src, arm, cell, seed, sim, collapsed (its verdict's),
                 end_ms (the run's last hold: a never-held chunk's
                 censoring time)
      fs_arr     the jobs' arrivals
      fs_seed    k, node: the seed (the node that published the most)
      fs_pub     k, seq, pub_ms (the seed's hold), first_ms (another
                 node's first)
      fs_rate    k, ms_per_seq: the publishing interval per chunk
      fs_holds   k, node, seq, incarnation, held_ms, how (first holds)
      fs_phase   k, node, sec, phase: each viewer's phase by second
      fs_all     a view of every row, picked and rivals, with truth, the
                 requester's phase and whether `sel` keeps it (`kept`;
                 every section reads only those, but `aligned`)
      fs_asks    the asked rows (module docs), with their structure
    """
    return f"""{jobs_sql(sel, "fs_sel")}
CREATE OR REPLACE TEMP TABLE fs_arr AS
SELECT x.k, x.node, x.seq, x.incarnation, x.t_ms, x.how FROM arrivals x SEMI JOIN fs_sel USING (k, src);
CREATE OR REPLACE TEMP TABLE fs_jobs AS
SELECT j.*, e.end_ms FROM fs_sel j LEFT JOIN (SELECT k, max(t_ms) AS end_ms FROM fs_arr GROUP BY k) e USING (k);
CREATE OR REPLACE TEMP TABLE fs_seed AS
SELECT k, arg_max(node, n) AS node FROM (SELECT k, node, count(*) AS n FROM fs_arr WHERE how = 'own' GROUP BY ALL)
GROUP BY k;
CREATE OR REPLACE TEMP TABLE fs_pub AS
SELECT x.k, x.seq, min(x.t_ms) FILTER (WHERE x.node = s.node) AS pub_ms,
       min(x.t_ms) FILTER (WHERE x.node <> s.node) AS first_ms
FROM fs_arr x JOIN fs_seed s USING (k) GROUP BY ALL;
CREATE OR REPLACE TEMP TABLE fs_rate AS
SELECT k, (max(pub_ms) - min(pub_ms)) / nullif(max(seq) - min(seq), 0) AS ms_per_seq
FROM fs_pub WHERE pub_ms IS NOT NULL GROUP BY k;
CREATE OR REPLACE TEMP TABLE fs_holds AS
SELECT k, node, seq, incarnation, min(t_ms) AS held_ms, arg_min(how, t_ms) AS how FROM fs_arr GROUP BY ALL;
CREATE OR REPLACE TEMP TABLE fs_phase AS
SELECT v.k, v.node, unnest(range(p.start, p.stop)) AS sec, p.phase
FROM viewers v SEMI JOIN fs_sel USING (k, src)
JOIN viewer_phases p ON p.k = v.k AND p.src = v.src AND p.viewer = v.viewer;
CREATE OR REPLACE TEMP VIEW fs_all AS
SELECT j.arm, j.cell, j.seed, j.sim, j.end_ms, a.k, a.t_ms, a.node, a.ask, a.round, a.picked, a.declares,
       a.peer, a.stream, a.seq, coalesce(a.grace_ms, 0) AS grace_ms, a.rtt_ms,
       a.predicted_ms AS p50, a.predicted_p90_ms AS p90, a.predicted_p99_ms AS p99,
       a.frontier_seq, a.hole_seq, a.state_age_ms, a.outcome, a.reason, a.t_outcome_ms, a.peer_incarnation,
       a.held_ms, a.pred_err_ms, a.t_ms + a.rtt_ms // 2 AS arrive_ms,
       a.t_ms + a.rtt_ms // 2 + coalesce(a.grace_ms, 0) AS tau_ms, coalesce(p.phase, {q(vocab.TAIL)}) AS phase,
       {phase_filter(sel, "p.phase")} AS kept
FROM asks a JOIN fs_jobs j USING (k, src)
LEFT JOIN fs_phase p ON p.k = a.k AND p.node = a.node AND p.sec = a.t_ms // 1000
WHERE a.honest AND a.peer_honest AND (a.held_ms IS NULL OR a.live);
CREATE OR REPLACE TEMP TABLE fs_asks AS
SELECT a.*, coalesce(h.how, 'never') AS path,
       CASE WHEN u.pub_ms IS NULL OR u.pub_ms > a.t_ms THEN 'unpublished'
            WHEN u.first_ms IS NULL OR u.first_ms > a.t_ms THEN 'seed_only' ELSE 'spread' END AS supply,
       {position_sql("a")} AS position,
       CASE WHEN coalesce(o.elsewhere, false) THEN 'declared_elsewhere' ELSE 'not_elsewhere' END AS elsewhere,
       a.outcome IN ('not_yet', 'expired') AS fail, u.pub_ms
FROM fs_all a
LEFT JOIN fs_holds h ON h.k = a.k AND h.node = a.peer AND h.seq = a.seq AND h.incarnation = a.peer_incarnation
LEFT JOIN fs_pub u ON u.k = a.k AND u.seq = a.seq
LEFT JOIN (
    SELECT r.k, r.ask, bool_or({position_sql("r")} = 'declared') AS elsewhere
    FROM fs_all r JOIN fs_seed s USING (k) WHERE NOT r.picked AND r.peer <> s.node GROUP BY ALL
) o ON o.k = a.k AND o.ask = a.ask
WHERE a.picked;
"""


def prepare(conn, sel):
    """Makes the study's tables for `sel` on `conn` (prepare_sql)."""
    for s in conn.extract_statements(prepare_sql(sel)):
        conn.execute(s)


@contextlib.contextmanager
def connected(conn=None, threads=8, memory="16GB"):
    """`conn`, else a reader of the catalog of its own (simlab.catalog's),
    closed after the block."""
    if conn is not None:
        yield conn
        return
    from . import catalog

    with catalog.connected(threads=threads, memory=memory) as c:
        # The decisions' every row, rivals and all, is tens of millions in
        # the overload: no section needs them in order.
        c.execute("SET preserve_insertion_order = false")
        yield c


# --- the sections ---------------------------------------------------------------------


def wis(y, row="a"):
    """SQL: the quantile form of WIS (seconds) for outcome `y`: the mean of
    twice the pinball losses of the three logged quantiles."""
    return "(" + " + ".join(f"2 * {pinball(y, f'{row}.{c}', t)}" for c, t in QUANTILES) + f") / {len(QUANTILES)}"


def outcome_ms(row="a"):
    """SQL: a row's outcome for the scores: the peer's hold, or for a hold
    never seen its censoring time (the run's last hold)."""
    return f"coalesce({row}.held_ms, greatest({row}.end_ms, {row}.p99))"


def cover(row, qcol):
    """SQL: the share of a group's rows held by quantile `qcol` (as the
    asks view's held_by_* reads it: held at or before it), a never-held
    row uncovered, over the rows that have that quantile."""
    return f"avg(coalesce({row}.held_ms <= {row}.{qcol}, false)::INT) FILTER (WHERE {row}.{qcol} IS NOT NULL)"


def score_cols(row="a"):
    """SQL: the score columns of a group of rows with predictions (module
    docs: scores)."""
    y = outcome_ms(row)
    held = f"{row}.held_ms IS NOT NULL"
    pin = ", ".join(f"avg({pinball(y, f'{row}.{c}', t)}) AS pinball_{c}" for c, t in QUANTILES)
    return f"""count(*) AS n, avg((NOT {held})::INT) AS never,
       {pin}, avg({wis(y, row)}) AS wis,
       {cover(row, "p50")} AS cover_p50,
       {cover(row, "p90")} AS cover_p90,
       {cover(row, "p99")} AS cover_p99,
       avg(({held} AND {row}.held_ms < {row}.t_ms)::INT) AS before_decision,
       sketch_quantile({row}.held_ms - {row}.p50, 50) AS err_p50_ms,
       sketch_quantile({row}.held_ms - {row}.p50, 90) AS err_p90_ms,
       sketch_quantile({row}.p90 - {row}.p50, 50) AS spread_p90_ms,
       sketch_quantile({row}.p99 - {row}.p50, 50) AS spread_p99_ms,
       avg(({row}.p99 = {row}.p50)::INT) AS no_spread"""


def pit_cols(prefix=""):
    """SQL over score_cols' columns: the PIT histogram's bins (below p50,
    p50-p90, p90-p99, past p99 or never) and the D-calibration statistic,
    the chi-square distance of the bins from PIT_EXPECTED (0 when
    calibrated; times n it is chi-square with 3 degrees of freedom)."""
    p = prefix
    bins = [f"{p}cover_p50", f"{p}cover_p90 - {p}cover_p50", f"{p}cover_p99 - {p}cover_p90", f"1 - {p}cover_p99"]
    cols = [f"{b} AS pit_{i}" for i, b in enumerate(bins)]
    dcal = " + ".join(f"power({b} - {e}, 2) / {e}" for b, e in zip(bins, PIT_EXPECTED))
    return ", ".join(cols) + f", {dcal} AS dcal"


def decompose_sql():
    # DuckDB's greatest and least skip NULLs: a row without an error keeps none.
    clip = f"CASE WHEN a.held_ms - a.p50 IS NOT NULL THEN greatest(-{CLIP_MS}, least({CLIP_MS}, a.held_ms - a.p50)) END"
    return f"""
WITH x AS (
    SELECT a.arm, a.cell, a.round, a.fail, a.outcome = 'not_yet' AS not_yet, a.held_ms, a.p50,
           {clip} AS e, unnest({list(STRUCTURE)!r}) AS factor,
           unnest([{', '.join(f'a.{c}' for c in STRUCTURE)}]) AS level
    FROM fs_asks a WHERE a.kept
), lv AS (
    SELECT arm, cell, round, factor, level, count(*) AS n, count(*) FILTER (WHERE fail) AS fails,
           count(*) FILTER (WHERE not_yet) AS not_yets, count(e) AS held, avg(e) AS mean_e,
           sum(abs(e)) AS abs_e, sketch_quantile(held_ms - p50, 50) AS err_p50_ms
    FROM x GROUP BY ALL
), tot AS (
    SELECT arm, cell, round, factor, sum(n) AS n, sum(fails) AS fails, sum(abs_e) AS abs_e,
           sum(held * mean_e) / nullif(sum(held), 0) AS mean_e
    FROM lv GROUP BY ALL
), ss AS (
    SELECT x.arm, x.cell, x.round, x.factor, sum(power(x.e - t.mean_e, 2)) AS ss_total
    FROM x JOIN tot t USING (arm, cell, round, factor) WHERE x.e IS NOT NULL GROUP BY ALL
), sb AS (
    SELECT l.arm, l.cell, l.round, l.factor, sum(l.held * power(l.mean_e - t.mean_e, 2)) AS ss_between
    FROM lv l JOIN tot t USING (arm, cell, round, factor) WHERE l.held > 0 GROUP BY ALL
)
SELECT l.arm, l.cell, l.round, l.factor, l.level, l.n, l.n / t.n AS share, l.fails / l.n AS fail_rate,
       l.fails / nullif(t.fails, 0) AS fail_share, l.not_yets / l.n AS not_yet_rate, l.held,
       l.err_p50_ms, l.abs_e / nullif(t.abs_e, 0) AS abs_err_share,
       sb.ss_between / nullif(ss.ss_total, 0) AS eta2
FROM lv l JOIN tot t USING (arm, cell, round, factor)
LEFT JOIN ss USING (arm, cell, round, factor) LEFT JOIN sb USING (arm, cell, round, factor)
ORDER BY l.arm, l.cell, l.round, l.factor, l.n DESC;
"""


# The failure predictors `explained` scores, each a class of a row whose
# failure rate (in the other seeds) is its prediction; `model` is the
# model's own probability, `base` the rate of every row.
EXPLAINERS = {
    "base": "'all'",
    "model_cal": "floor(pm * 10)::VARCHAR",
    "position": "position",
    "path": "path",
    "supply": "supply",
    "elsewhere": "elsewhere",
    "observable": "position || '/' || elsewhere",
    "observable_model": "position || '/' || elsewhere || '/' || floor(pm * 5)::VARCHAR",
    "joint": "path || '/' || supply || '/' || position",
    "joint_model": "path || '/' || supply || '/' || position || '/' || floor(pm * 5)::VARCHAR",
}


def explained_sql(explainers=None, source="fs_asks", cols=STRUCTURE):
    """The explained section's statement: each of `explainers` ({name: a
    class's SQL over the columns `cols` and the model's probability `pm`},
    EXPLAINERS by default) scored as a predictor of failure over the kept
    asked rows of `source`, beside the model's own."""
    explainers = explainers or EXPLAINERS
    pm = f"greatest({EPS}, least({1 - EPS}, 1 - {cdf('a.tau_ms')}))"
    names = ", ".join(q(n) for n in explainers)
    return f"""
WITH r AS (
    SELECT a.arm, a.cell, a.round, a.seed, a.fail::INT AS y, {pm} AS pm, {', '.join(f'a.{c}' for c in cols)}
    FROM {source} a WHERE a.kept AND a.p50 IS NOT NULL AND a.outcome IS NOT NULL AND a.outcome <> 'cancelled'
), x AS (
    SELECT arm, cell, round, seed, y, unnest([{names}]) AS pred,
           unnest([{', '.join(explainers.values())}]) AS cls FROM r
), c AS (
    SELECT arm, cell, round, pred, cls, seed, count(*) AS n, sum(y) AS f FROM x GROUP BY ALL
), ct AS (
    SELECT arm, cell, round, pred, cls, sum(n) AS n, sum(f) AS f FROM c GROUP BY ALL
), b AS (
    SELECT arm, cell, round, seed, n, f FROM c WHERE pred = 'base'
), bt AS (
    SELECT arm, cell, round, sum(n) AS n, sum(f) AS f FROM b GROUP BY ALL
), loso AS (
    -- A class's rate in the other seeds, pulled toward their base rate by
    -- one row's weight (a class only this seed has takes the base rate).
    SELECT c.*, (ct.f - c.f + (bt.f - b.f) / nullif(bt.n - b.n, 0)) / (ct.n - c.n + 1) AS rate
    FROM c JOIN ct USING (arm, cell, round, pred, cls) JOIN b USING (arm, cell, round, seed)
    JOIN bt USING (arm, cell, round)
), scored AS (
    SELECT arm, cell, round, pred, sum(n) AS n,
           -sum(f * ln(greatest({EPS}, least({1 - EPS}, rate)))
                + (n - f) * ln(greatest({EPS}, least({1 - EPS}, 1 - rate)))) / sum(n) AS log_loss,
           sum(f * power(1 - rate, 2) + (n - f) * power(rate, 2)) / sum(n) AS brier
    FROM loso WHERE rate IS NOT NULL GROUP BY ALL
    UNION ALL
    SELECT arm, cell, round, 'model', count(*), -avg(y * ln(pm) + (1 - y) * ln(1 - pm)), avg(power(y - pm, 2))
    FROM r GROUP BY ALL
)
SELECT s.arm, s.cell, s.round, s.pred, s.n, bt.f / bt.n AS fail_rate, s.log_loss, s.brier,
       1 - s.log_loss / nullif(b0.log_loss, 0) AS r2, 1 - s.brier / nullif(b0.brier, 0) AS brier_skill
FROM scored s JOIN scored b0 ON b0.arm = s.arm AND b0.cell = s.cell AND b0.round = s.round AND b0.pred = 'base'
JOIN bt ON bt.arm = s.arm AND bt.cell = s.cell AND bt.round = s.round
ORDER BY s.arm, s.cell, s.round, r2 DESC;
"""


# The decoded state's explainers (simlab.regimes): the asked peer's state
# at the decision's second, alone and with the chunk's position, beside
# the structural classes.
STATE_EXPLAINERS = {
    "base": "'all'",
    "model_cal": "floor(pm * 10)::VARCHAR",
    "position": "position",
    "joint": "path || '/' || supply || '/' || position",
    "state": "state",
    "state_position": "state || '/' || position",
    "state_model": "state || '/' || floor(pm * 5)::VARCHAR",
}


def state_explained_sql():
    """explained's statements over the asked rows whose peer's stream
    simlab.regimes decoded (fs_states, regimes.load_states), with that
    state as a class (STATE_EXPLAINERS)."""
    return f"""
CREATE OR REPLACE TEMP TABLE fs_asks_states AS
SELECT a.*, s.state FROM fs_asks a JOIN fs_states s ON s.k = a.k AND s.peer = a.peer AND s.stream = a.stream
     AND s.sec = a.t_ms // 1000;
{explained_sql(STATE_EXPLAINERS, "fs_asks_states", (*STRUCTURE, "state"))}"""


def agreement_sql():
    """How much a decision's failure is the asked peer's moment against
    the requester's: the intraclass correlation ICC(1) of the kept asked
    rows' failures grouped by (run, peer, stream, second) and by (run,
    requester, stream, second), over groups of two rows or more."""
    return """
WITH g AS (
    SELECT arm, cell, 'peer' AS grouping, k, peer AS member, stream, t_ms // 1000 AS sec, fail::INT AS y
    FROM fs_asks WHERE kept AND outcome IS NOT NULL AND outcome <> 'cancelled'
    UNION ALL
    SELECT arm, cell, 'requester', k, node, stream, t_ms // 1000, fail::INT
    FROM fs_asks WHERE kept AND outcome IS NOT NULL AND outcome <> 'cancelled'
), s AS (
    SELECT arm, cell, grouping, k, member, stream, sec, count(*) AS n, avg(y) AS m, sum(y) AS f
    FROM g GROUP BY ALL HAVING count(*) >= 2
), t AS (
    SELECT arm, cell, grouping, sum(n) AS n, count(*) AS groups, sum(f) / sum(n) AS m FROM s GROUP BY ALL
), ms AS (
    SELECT s.arm, s.cell, s.grouping, sum(s.n * power(s.m - t.m, 2)) / (any_value(t.groups) - 1) AS msb,
           sum(s.f * power(1 - s.m, 2) + (s.n - s.f) * power(s.m, 2)) / (any_value(t.n) - any_value(t.groups))
               AS msw,
           any_value(t.n) / any_value(t.groups) AS n_bar, any_value(t.groups) AS groups, any_value(t.n) AS rows
    FROM s JOIN t USING (arm, cell, grouping) GROUP BY ALL
)
SELECT arm, cell, grouping, groups, rows, n_bar, (msb - msw) / nullif(msb + (n_bar - 1) * msw, 0) AS icc
FROM ms ORDER BY ALL;
"""


def scores_sql():
    return f"""
WITH g AS (
    SELECT a.arm, a.cell, a.round, 'asked' AS rows, a.phase, {score_cols()}
    FROM fs_asks a WHERE a.kept AND a.p50 IS NOT NULL GROUP BY ALL
    UNION ALL
    SELECT a.arm, a.cell, a.round, 'every', 'kept', {score_cols()}
    FROM fs_all a WHERE a.kept AND a.p50 IS NOT NULL GROUP BY ALL
)
SELECT *, {pit_cols()} FROM g ORDER BY arm, cell, round, rows, phase;
"""


def aligned_sql():
    kinds = ", ".join(q(k) for k in ALIGN_KINDS)
    span = f"a.t_ms - e.t_ms >= -{ALIGN_BEFORE_S * 1000} AND a.t_ms - e.t_ms < {ALIGN_AFTER_S * 1000}"
    return f"""
WITH ev AS (
    SELECT e.k, e.kind, min(e.t_ms) AS t_ms FROM events e SEMI JOIN fs_jobs USING (k, src)
    WHERE e.kind IN ({kinds}) AND e.t_ms > 0 GROUP BY ALL
), pc AS (
    SELECT p.k, p.node, p.stream, p.t_ms FROM parents p SEMI JOIN fs_jobs USING (k, src) WHERE p.t_ms > 0
), run_level AS (
    SELECT a.*, e.kind, a.t_ms - e.t_ms AS off_ms FROM fs_asks a JOIN ev e ON e.k = a.k WHERE {span}
), per_peer AS (
    SELECT a.*, 'parent_change' AS kind, a.t_ms - e.t_ms AS off_ms
    FROM fs_asks a ASOF JOIN pc e ON e.k = a.k AND e.node = a.peer AND e.stream = a.stream
         AND a.t_ms + {ALIGN_BEFORE_S * 1000} >= e.t_ms
    WHERE {span}
), joins AS (
    SELECT a.*, w.kind, a.t_ms - v.arrive_ms AS off_ms
    FROM fs_asks a JOIN (VALUES ('receiver_join'), ('peer_join')) w(kind) ON true
    JOIN viewers v ON v.k = a.k AND v.node = CASE w.kind WHEN 'receiver_join' THEN a.node ELSE a.peer END
    WHERE v.arrive_ms > 0 AND a.t_ms - v.arrive_ms >= -{ALIGN_BEFORE_S * 1000}
          AND a.t_ms - v.arrive_ms < {ALIGN_AFTER_S * 1000}
), x AS (SELECT * FROM run_level UNION ALL BY NAME SELECT * FROM per_peer UNION ALL BY NAME SELECT * FROM joins)
SELECT a.arm, a.cell, a.kind, (floor(a.off_ms / {ALIGN_BIN_S * 1000}) * {ALIGN_BIN_S})::INT AS off_s,
       count(*) AS n, avg(a.fail::INT) AS fail_rate, count(a.p50) AS predicted,
       {cover("a", "p50")} AS cover_p50,
       {cover("a", "p90")} AS cover_p90,
       sketch_quantile(a.held_ms - a.p50, 50) AS err_p50_ms,
       avg({wis(outcome_ms())}) FILTER (WHERE a.p50 IS NOT NULL) AS wis
FROM x a GROUP BY ALL ORDER BY a.arm, a.cell, a.kind, off_s;
"""


def baselines_sql():
    ps = ", ".join(str(round(t * 100)) for _, t in QUANTILES)
    model_wis = " + ".join(f"2 * {pinball('b.held_ms', f'b.m{c[1:]}', t)}" for c, t in QUANTILES)
    return f"""
WITH r AS (
    SELECT a.arm, a.cell, a.round, a.seed, a.t_ms, a.held_ms, a.p50, a.p90, a.p99,
           coalesce(a.pub_ms, f.pub_ms + (a.seq - a.frontier_seq) * rt.ms_per_seq) + (hf.held_ms - f.pub_ms)
               AS persistence,
           hf.held_ms + (a.seq - a.frontier_seq) * rt.ms_per_seq AS cadence
    FROM fs_asks a
    JOIN fs_holds hf ON hf.k = a.k AND hf.node = a.peer AND hf.seq = a.frontier_seq
         AND hf.incarnation = a.peer_incarnation
    JOIN fs_pub f ON f.k = a.k AND f.seq = a.frontier_seq
    JOIN fs_rate rt ON rt.k = a.k
    WHERE a.kept AND a.held_ms IS NOT NULL AND a.p50 IS NOT NULL AND f.pub_ms IS NOT NULL
), long AS (
    SELECT arm, cell, round, seed, t_ms, held_ms, p50, p90, p99, unnest(['persistence', 'cadence']) AS base,
           unnest([persistence, cadence]) AS point FROM r
), per_seed AS (
    SELECT arm, cell, round, base, seed, sketch_quantiles(held_ms - point, [{ps}]) AS qs
    FROM long GROUP BY ALL
), loso AS (
    SELECT s.arm, s.cell, s.round, s.base, s.seed, sketch_quantile(o.qs[1], 50) AS q1,
           sketch_quantile(o.qs[2], 50) AS q2,
           sketch_quantile(o.qs[3], 50) AS q3
    FROM per_seed s JOIN per_seed o ON o.arm = s.arm AND o.cell = s.cell AND o.round = s.round
         AND o.base = s.base AND o.seed <> s.seed
    GROUP BY ALL
), b AS (
    SELECT l.arm, l.cell, l.round, l.base, l.held_ms, l.t_ms, l.p50 AS m50, l.p90 AS m90, l.p99 AS m99,
           l.point + q.q1 AS p50, l.point + q.q2 AS p90, l.point + q.q3 AS p99
    FROM long l JOIN loso q USING (arm, cell, round, base, seed)
), scored AS (
    SELECT arm, cell, round, base, count(*) AS n,
           avg({wis('b.held_ms', 'b')}) AS wis, avg(({model_wis}) / {len(QUANTILES)}) AS model_wis,
           avg({pinball('b.held_ms', 'b.p50', 0.5)}) AS pinball_p50,
           avg({pinball('b.held_ms', 'b.m50', 0.5)}) AS model_pinball_p50,
           avg((held_ms <= p50)::INT) AS cover_p50, avg((held_ms <= p90)::INT) AS cover_p90,
           avg((held_ms <= p99)::INT) AS cover_p99,
           avg((held_ms <= m50)::INT) AS model_cover_p50, avg((held_ms <= m90)::INT) AS model_cover_p90,
           sketch_quantile(held_ms - p50, 50) AS err_p50_ms, sketch_quantile(held_ms - m50, 50) AS model_err_p50_ms
    FROM b GROUP BY ALL
)
SELECT *, 1 - model_wis / nullif(wis, 0) AS skill, 1 - model_pinball_p50 / nullif(pinball_p50, 0) AS skill_p50
FROM scored ORDER BY arm, cell, round, base;
"""


def bucket(col, edges, unit=""):
    """SQL: the label of `col`'s bucket among lower `edges` (a NULL
    'none')."""
    whens = " ".join(f"WHEN {col} >= {e} THEN '{i + 1:02d} >={e}{unit}'"
                     for i, e in reversed(list(enumerate(edges))))
    return f"CASE WHEN {col} IS NULL THEN 'none' {whens} ELSE '00 <{edges[0]}{unit}' END"


def staleness_sql():
    return f"""
WITH pc AS (
    SELECT p.k, p.node, p.stream, p.t_ms FROM parents p SEMI JOIN fs_jobs USING (k, src)
), x AS (
    SELECT a.*, 'state_age' AS axis, {bucket('a.state_age_ms', STATE_AGES_MS, 'ms')} AS bucket
    FROM fs_asks a WHERE a.kept
    UNION ALL
    SELECT a.*, 'since_parent_change', {bucket('(a.t_ms - e.t_ms) / 1000', BREAK_AGES_S, 's')}
    FROM fs_asks a ASOF LEFT JOIN pc e ON e.k = a.k AND e.node = a.peer AND e.stream = a.stream
         AND a.t_ms >= e.t_ms
    WHERE a.kept
)
SELECT a.arm, a.cell, a.round, a.axis, a.bucket, count(*) AS n, avg(a.fail::INT) AS fail_rate,
       count(a.p50) AS predicted, sketch_quantile(a.held_ms - a.p50, 50) AS err_p50_ms,
       avg(abs(a.held_ms - a.p50)) AS abs_err_ms,
       {cover("a", "p50")} AS cover_p50,
       {cover("a", "p90")} AS cover_p90,
       avg({wis(outcome_ms())}) FILTER (WHERE a.p50 IS NOT NULL) AS wis
FROM x a GROUP BY ALL ORDER BY a.arm, a.cell, a.round, a.axis, a.bucket;
"""


def selection_sql(where="true"):
    return f"""
WITH r AS (
    SELECT a.arm, a.cell, a.round, a.k, a.ask, a.picked, a.held_ms - a.p50 AS e, a.p50, a.held_ms, a.arrive_ms,
           a.tau_ms, a.peer = s.node AS seed
    FROM fs_all a JOIN fs_seed s USING (k) WHERE {where} AND a.kept AND (a.picked OR a.p50 IS NOT NULL)
), per AS (
    SELECT arm, cell, round, k, ask,
           any_value(e) FILTER (WHERE picked) AS e_pick,
           avg(e) FILTER (WHERE NOT picked) AS e_rivals,
           count(*) FILTER (WHERE NOT picked) AS rivals,
           coalesce(bool_or(held_ms IS NULL OR held_ms > tau_ms) FILTER (WHERE picked), false) AS pick_missed,
           coalesce(bool_or(held_ms <= arrive_ms) FILTER (WHERE NOT picked), false) AS rival_held,
           coalesce(bool_or(held_ms <= arrive_ms) FILTER (WHERE NOT picked AND seed), false) AS seed_held,
           coalesce(bool_or(held_ms <= arrive_ms) FILTER (WHERE NOT picked AND NOT seed), false) AS other_held,
           bool_or(seed) FILTER (WHERE picked) AS pick_seed,
           min(p50) FILTER (WHERE NOT picked) < any_value(p50) FILTER (WHERE picked) AS rival_sooner
    FROM r GROUP BY ALL
)
SELECT arm, cell, round, count(*) AS asks, avg(rivals) AS rivals, avg(pick_seed::INT) AS pick_seed,
       avg(e_pick) AS err_pick_ms, avg(e_rivals) AS err_rivals_ms,
       sketch_quantile(e_pick, 50) AS err_pick_p50_ms, sketch_quantile(e_rivals, 50) AS err_rivals_p50_ms,
       avg(e_pick - e_rivals) AS curse_ms, avg(pick_missed::INT) AS pick_missed,
       avg((pick_missed AND rival_held)::INT) AS regret, avg((pick_missed AND other_held)::INT) AS regret_peer,
       avg((pick_missed AND seed_held AND NOT other_held)::INT) AS regret_seed_only,
       avg(rival_sooner::INT) AS rival_predicted_sooner
FROM per WHERE rivals > 0 GROUP BY ALL ORDER BY ALL;
"""


OUTCOMES = ("answered", "not_yet", "expired", "elsewhere", "cancelled", "closed")


# A request that failed: refused for now, timed out, or overtaken by the
# chunk from elsewhere.
FAILED = ("not_yet", "expired", "elsewhere")


def bound_sql(have_haves, kept="true"):
    nulls = ", ".join(f"NULL::DOUBLE AS {o}" for o in (*OUTCOMES, "failed", "failed_seed_min", "failed_seed_max"))
    # The HAVEs are hundreds of millions: their quantiles come from a
    # histogram of whole ms (each the least delay its cumulative share
    # reaches).
    haves = f"""
    SELECT arm, cell, 'have delay' AS what, any_value(tot) AS n,
           [min(delay_ms) FILTER (WHERE cum >= 0.5 * tot), min(delay_ms) FILTER (WHERE cum >= 0.9 * tot),
            min(delay_ms) FILTER (WHERE cum >= 0.99 * tot)]::DOUBLE[] AS q_ms, {nulls}
    FROM (
        SELECT *, sum(n) OVER (PARTITION BY arm, cell ORDER BY delay_ms) AS cum,
               sum(n) OVER (PARTITION BY arm, cell) AS tot
        FROM (
            SELECT j.arm, j.cell, h.delay_ms, count(*) AS n
            FROM haves h JOIN fs_jobs j USING (k, src)
            LEFT JOIN fs_phase p ON p.k = h.k AND p.node = h.node AND p.sec = h.t_ms // 1000
            WHERE h.honest AND h.peer_honest AND h.live AND {kept}
            GROUP BY ALL
        )
    ) GROUP BY ALL
    UNION ALL""" if have_haves else ""
    shares = ", ".join(f"avg((a.outcome = {q(o)})::INT) AS {o}" for o in OUTCOMES)
    failed = f"a.outcome IN ({', '.join(q(o) for o in FAILED)})"
    return f"""
WITH per_seed AS (
    SELECT arm, cell, round, seed, avg(({failed})::INT) AS f FROM fs_asks a WHERE a.kept GROUP BY ALL
), spread AS (
    SELECT arm, cell, round, min(f) AS lo, max(f) AS hi FROM per_seed GROUP BY ALL
)
SELECT * FROM ({haves}
    SELECT a.arm, a.cell, 'ask ' || a.round AS what, count(*) AS n,
           sketch_quantiles(CASE WHEN a.outcome = 'answered' THEN a.t_outcome_ms - a.t_ms END, [50, 90, 99]) AS q_ms,
           {shares}, avg(({failed})::INT) AS failed, any_value(s.lo) AS failed_seed_min,
           any_value(s.hi) AS failed_seed_max
    FROM fs_asks a JOIN spread s USING (arm, cell, round) WHERE a.kept GROUP BY ALL
) ORDER BY ALL;
"""


def runs_sql():
    return """
WITH per AS (
    SELECT j.arm, j.cell, j.seed, j.collapsed, count(a.k) AS asks, count(a.k) FILTER (WHERE a.fail) AS fails,
           count(a.k) FILTER (WHERE a.round = 'flight') AS flight,
           count(a.k) FILTER (WHERE a.supply = 'seed_only') AS seed_only,
           count(a.k) FILTER (WHERE a.position IN ('hole', 'behind')) AS hole_behind
    FROM fs_jobs j LEFT JOIN fs_asks a ON a.k = j.k AND a.arm = j.arm AND a.cell = j.cell AND a.kept
    GROUP BY ALL
)
SELECT arm, cell, collapsed, count(*) AS runs, list(seed ORDER BY seed) AS seeds, sum(asks) AS asks,
       CASE WHEN count(asks) > 0 THEN [min(asks), sketch_quantile(asks, 50), max(asks)] END AS asks_per_run,
       sum(fails) / nullif(sum(asks), 0) AS fail_rate,
       sum(flight) / nullif(sum(asks), 0) AS flight_share, sum(seed_only) / nullif(sum(asks), 0) AS seed_only,
       sum(hole_behind) / nullif(sum(asks), 0) AS hole_behind
FROM per GROUP BY ALL ORDER BY ALL;
"""


SECTION_SQL = {"runs": runs_sql, "decompose": decompose_sql, "explained": explained_sql, "scores": scores_sql,
               "aligned": aligned_sql, "baselines": baselines_sql, "staleness": staleness_sql,
               "selection": selection_sql}


def tables(conn):
    """The catalog's tables and views (the main schema's)."""
    return {r[0] for r in conn.execute("SELECT table_name FROM information_schema.tables").fetchall()}


def report(sel, sections=SECTIONS, conn=None, say=None):
    """Each section's Result over `sel`'s jobs (module docs), in order."""
    out = {}
    with connected(conn) as c:
        prepare(c, sel)
        for s in sections:
            if say:
                say(s)
            sql = (bound_sql("haves" in tables(c), phase_filter(sel, "p.phase")) if s == "bound"
                   else SECTION_SQL[s]())
            out[s] = engine.run(c, sql)
    return out


def fmt(v):
    """A report value as text: numbers to three significant figures."""
    if v is None:
        return None
    if isinstance(v, float):
        return f"{v:.3g}" if abs(v) < 1000 else f"{v:.0f}"
    if isinstance(v, list):
        return " ".join(fmt(x) or "-" for x in v)
    return str(v)


def report_text(results):
    """The report's text: each section's table."""
    out = []
    for name, r in results.items():
        out.append(f"## {name}\n")
        out.append(engine.render_table(r.names, [[fmt(v) for v in row] for row in r.values()]))
        out.append("\n")
    return "".join(out)
