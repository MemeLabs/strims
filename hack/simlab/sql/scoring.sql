-- Copyright 2026 Strims contributors
-- SPDX-License-Identifier: AGPL-3.0-only
--
-- suite.py's scoring as the catalog's macros: what its scorecards, arm
-- scores and races compute, here once (suite.py prints what they return).
-- catalog.py loads them (scoring_sql) after what it makes itself:
-- metric_defs, job_runs, job_metrics and job_metrics_of from suite.py's
-- METRICS, and t975, Student's t. It fills in each @{...}: a constant of
-- suite.py's, analyze.py's or its own, or a row field (r.<field>), the
-- field where the rows have it, else NULL. A macro's `-- needs:` line names
-- the tables (or table.column) it reads that a catalog indexed by older
-- code may lack: a session's own objects over such a catalog make it one
-- that fails saying so (catalog.needed).
--
-- A side is one arm's runs, a list of {'cell', 'seed', 'k', 'src'}
-- structs: the job k is that arm's run of the cell at the seed, and src
-- the source of the run's own copy (a farm index line's flush, a local
-- invocation's), whose host numbers (job_runs' wall_s, rss_mb) it reads,
-- or NULL for the job's (jobs.src); a job's row and verdict are its own
-- source's, which every copy shares. Two sides pair by (cell, seed): the
-- seeds of A in a cell that B has too (calibrate pairs one arm's seeds
-- N+1..2N with its 1..N by naming them 1..N). A side's order is its runs'
-- order: traces_of breaks ties between cells by it.
-- Only runs whose row the catalog holds count. arm_side(exp, arm) is an
-- experiment's arm as a side (experiment_jobs), and exp_cells(exp_a, exp_b)
-- the cells of exp_b that exp_a has, in exp_b's order: pairs, effects,
-- collapse_test, traces, arm_cells and arm_score take (exp_a, arm_a,
-- exp_b, arm_b) and score arm_b of exp_b against arm_a of exp_a over those
-- cells, as `suite.py score exp_a:arm_a exp_b:arm_b` does; the *_of macros
-- take a cell list and two sides.
--
--   SELECT cell, metric, effect, lo, hi, mark FROM effects('s8ed1e', 'p4tf', 's8ed1e', 'vouch')
--   WHERE mark <> ' ' ORDER BY cell_ord, ord;

-- Student's t at 97.5 % at df_ degrees of freedom: t975's (catalog.py's
-- t975, df 1 to @{t975_df}), and past them the Cornish-Fisher expansion in
-- 1/df (Abramowitz and Stegun 26.7.5) to 1/df^4, within 1e-10 there.
CREATE OR REPLACE MACRO t975_at(df_) AS
    coalesce((SELECT t FROM t975 WHERE df = df_),
             @{t975_z} + @{t975_g1} / df_ + @{t975_g2} / pow(df_, 2) + @{t975_g3} / pow(df_, 3)
             + @{t975_g4} / pow(df_, 4));

-- The hypergeometric probability of x of hits_ in the first n_ of n_ + n2_.
CREATE OR REPLACE MACRO hypergeom_p(x_, n_, n2_, hits_) AS
    exp(lgamma(n_ + 1) - lgamma(x_ + 1) - lgamma(n_ - x_ + 1)
        + lgamma(n2_ + 1) - lgamma(hits_ - x_ + 1) - lgamma(n2_ - hits_ + x_ + 1)
        - lgamma(n_ + n2_ + 1) + lgamma(hits_ + 1) + lgamma(n_ + n2_ - hits_ + 1));

-- The two-sided Fisher exact p of k_ in n_ against m_ in n2_: the
-- probabilities, in order, of every split no likelier than the one seen.
CREATE OR REPLACE MACRO fisher_p(k_, n_, m_, n2_) AS
    least(1.0::DOUBLE, list_reduce(list_filter(
        list_transform(range(greatest(0, k_ + m_ - n2_), least(n_, k_ + m_) + 1),
                       lambda x: hypergeom_p(x, n_, n2_, k_ + m_)),
        lambda pr: pr <= hypergeom_p(k_, n_, n2_, k_ + m_) * (1 + 1e-9)), lambda acc, pr: acc + pr));

-- A paired effect's mark: + better, - worse, ~ moved where no direction is
-- better, ' ' a tie. It moved when its interval excludes no change (1 for
-- a ratio, 0 for a difference) and it passes the tie band (|ln x| past
-- ln(1 + tie), |x| past tie).
CREATE OR REPLACE MACRO effect_mark(better_, kind_, tie_, x_, lo_, hi_) AS
    CASE WHEN x_ IS NULL THEN ' '
         WHEN NOT CASE WHEN kind_ = 'ratio' THEN (lo_ > 1 OR hi_ < 1) AND abs(ln(x_)) > ln(1 + tie_)
                       ELSE (lo_ > 0 OR hi_ < 0) AND abs(x_) > tie_ END THEN ' '
         WHEN better_ IS NULL THEN '~'
         WHEN (CASE WHEN kind_ = 'ratio' THEN x_ > 1 ELSE x_ > 0 END) = (better_ = 'higher') THEN '+'
         ELSE '-' END;

-- A side's runs that the catalog holds a row of: cell, seed, k, src (the
-- job's source), run_src (the run's: src where the side names none) and
-- i, the run's place in the side.
CREATE OR REPLACE MACRO side_jobs(side) AS TABLE
    SELECT u.cell, u.seed::BIGINT AS seed, u.k, j.src, coalesce(u.src, j.src) AS run_src, u.i
    FROM (SELECT unnest(side, recursive := true), generate_subscripts(side, 1) AS i) u
    JOIN jobs j ON j.k = u.k
    SEMI JOIN rows r ON r.k = j.k AND r.src = j.src
    QUALIFY row_number() OVER (PARTITION BY u.cell, u.seed ORDER BY u.i) = 1;

-- Every metric both sides' runs at a cell and seed have a value of
-- (job_metrics_of): cell, seed, metric, a_value, b_value, and the runs'
-- jobs, a_k and b_k.
CREATE OR REPLACE MACRO pairs_of(a_jobs, b_jobs) AS TABLE
    WITH sa AS (FROM side_jobs(a_jobs)), sb AS (FROM side_jobs(b_jobs)),
    ab AS (
        SELECT sa.cell, sa.seed, sa.k AS a_k, sa.run_src AS a_src, sb.k AS b_k, sb.run_src AS b_src
        FROM sa JOIN sb ON sb.cell = sa.cell AND sb.seed = sa.seed),
    runs AS (SELECT cell, seed, a_k AS k, a_src AS src, true AS is_a FROM ab
             UNION ALL SELECT cell, seed, b_k, b_src, false FROM ab),
    vals AS (
        SELECT u.cell, u.seed, u.is_a, u.k, v.metric, v.value
        FROM runs u JOIN job_metrics_of(list_transform(a_jobs || b_jobs, lambda s: {'k': s.k, 'src': s.src})) v
          ON v.k = u.k AND v.src = u.src)
    SELECT cell, seed, metric, any_value(value) FILTER (WHERE is_a) AS a_value,
           any_value(value) FILTER (WHERE NOT is_a) AS b_value, any_value(k) FILTER (WHERE is_a) AS a_k,
           any_value(k) FILTER (WHERE NOT is_a) AS b_k
    FROM vals GROUP BY cell, seed, metric
    HAVING count(*) FILTER (WHERE is_a) > 0 AND count(*) FILTER (WHERE NOT is_a) > 0;

-- Each metric's paired effect of B against A in each cell of cell_list:
-- pairs (the seeds paired), a_median and b_median (each side's upper
-- median over them), n (the pairs it is over: a ratio's those with both
-- values above 0), effect (the geometric mean ratio, exp of the mean
-- ln(b/a), or the mean difference b - a), lo and hi (its 95 % t-interval;
-- one pair gives 0 or -inf to inf), mark (effect_mark; h for a host
-- metric that moved) and metric_defs' columns.
CREATE OR REPLACE MACRO effects_of(cell_list, a_jobs, b_jobs) AS TABLE
    WITH p AS (
        SELECT p.*, m.ord, m.kind, m.better, m.tie, m.host, m.shown,
               CASE WHEN m.kind = 'ratio' THEN CASE WHEN p.a_value > 0 AND p.b_value > 0
                                                    THEN ln(p.b_value / p.a_value) END
                    ELSE p.b_value - p.a_value END AS x
        FROM pairs_of(a_jobs, b_jobs) p JOIN metric_defs m ON m.name = p.metric
        WHERE list_contains(cell_list, p.cell)),
    g AS (
        SELECT cell, metric, any_value(ord) AS ord, any_value(kind) AS kind, any_value(better) AS better,
               any_value(tie) AS tie, any_value(host) AS host, any_value(shown) AS shown, count(*) AS pairs,
               list_sort(list(a_value))[count(*) // 2 + 1] AS a_median,
               list_sort(list(b_value))[count(*) // 2 + 1] AS b_median,
               list(x ORDER BY seed) FILTER (WHERE x IS NOT NULL) AS xs
        FROM p GROUP BY cell, metric),
    s AS (SELECT *, len(xs) AS n, list_reduce(xs, lambda acc, y: acc + y) / len(xs) AS mean FROM g),
    h AS (
        SELECT *, CASE WHEN n >= 2 THEN t975_at(n - 1)
                       * sqrt(list_reduce(list_transform(xs, lambda y: (y - mean) ** 2), lambda acc, y: acc + y)
                              / (n - 1)) / sqrt(n) END AS half
        FROM s),
    e AS (
        SELECT *,
               CASE WHEN kind = 'ratio' THEN exp(mean) ELSE mean END AS effect,
               CASE WHEN n IS NULL THEN NULL
                    WHEN n < 2 THEN CASE WHEN kind = 'ratio' THEN 0.0::DOUBLE ELSE '-inf'::DOUBLE END
                    WHEN kind = 'ratio' THEN exp(mean - half) ELSE mean - half END AS lo,
               CASE WHEN n IS NULL THEN NULL WHEN n < 2 THEN 'inf'::DOUBLE
                    WHEN kind = 'ratio' THEN exp(mean + half) ELSE mean + half END AS hi
        FROM h)
    SELECT cell, list_position(cell_list, cell) AS cell_ord, metric, ord, kind, better, tie, host, shown, pairs,
           a_median, b_median, n, effect, lo, hi,
           CASE WHEN host AND effect_mark(better, kind, tie, effect, lo, hi) <> ' ' THEN 'h'
                ELSE effect_mark(better, kind, tie, effect, lo, hi) END AS mark
    FROM e;

-- Each cell of cell_list's collapsed runs (verdicts.collapsed) over the
-- seeds both sides ran: ka of A's, kb of B's, n seeds, p their two-sided
-- Fisher exact p, mark + (fewer, p < 0.05), - (more), ' ' (p >= 0.05) or
-- NULL where neither collapsed.
CREATE OR REPLACE MACRO collapse_test_of(cell_list, a_jobs, b_jobs) AS TABLE
    WITH c AS (
        SELECT sa.cell, count(*) AS n, count(*) FILTER (WHERE va.collapsed) AS ka,
               count(*) FILTER (WHERE vb.collapsed) AS kb
        FROM side_jobs(a_jobs) sa JOIN side_jobs(b_jobs) sb ON sb.cell = sa.cell AND sb.seed = sa.seed
        LEFT JOIN verdicts va ON va.k = sa.k AND va.src = sa.src
        LEFT JOIN verdicts vb ON vb.k = sb.k AND vb.src = sb.src
        WHERE list_contains(cell_list, sa.cell) GROUP BY sa.cell)
    SELECT cell, list_position(cell_list, cell) AS cell_ord, ka, kb, n, fisher_p(ka, n, kb, n) AS p,
           CASE WHEN ka = 0 AND kb = 0 THEN NULL WHEN fisher_p(ka, n, kb, n) >= 0.05 THEN ' '
                WHEN kb < ka THEN '+' ELSE '-' END AS mark
    FROM c;

-- How many of each cell's runs both sides have, with a trace each, B ran
-- as A did (runs, same): the same trace, and the same reads where both
-- carry their digest (jobs.reads_hash). Every cell the sides share, not
-- only a cell list's; a_first is the cell's first place in A.
CREATE OR REPLACE MACRO traces_of(a_jobs, b_jobs) AS TABLE
    SELECT sa.cell, min(sa.i) AS a_first, count(*) AS runs,
           count(*) FILTER (WHERE ja.trace = jb.trace AND (coalesce(ja.reads_hash, '') = ''
                                                          OR coalesce(jb.reads_hash, '') = ''
                                                          OR ja.reads_hash = jb.reads_hash)) AS same
    FROM side_jobs(a_jobs) sa JOIN side_jobs(b_jobs) sb ON sb.cell = sa.cell AND sb.seed = sa.seed
    JOIN jobs ja ON ja.k = sa.k JOIN jobs jb ON jb.k = sb.k
    WHERE coalesce(ja.trace, '') <> '' AND coalesce(jb.trace, '') <> ''
    GROUP BY sa.cell;

-- An arm (B) against the base (A) cell by cell, as races and farm runs
-- rank arms: each cell of cell_list both ran, its collapses (ka, kb, n, p),
-- its lag p99 ratio (lag, lag_lo, lag_hi) and wire per useful byte ratio
-- (wire), and the guardrails it breaks there (breaks, in order):
--   collapsed    more collapsed runs (kb > ka) with p < 0.05;
--   lag_ceiling  ranked for overhead (overhead true), the base's median
--                lag p99 (a) under lag_ceiling and the arm's (b) not;
--   worse        a metric_defs guard 'worse' metric (delivery) marked -;
--   guard        in a cell the base never collapsed, a guard 'lo' metric's
--                ratio interval lying past guard_ratio (past lag_budget
--                when ranked for overhead: infinite, none, by default);
--   below_one    a guard 'hi' metric's (defector_gain) ratio interval
--                lying under 1: defecting pays more.
-- A break: {cell, kind, metric, x (the effect; lag_ceiling's ceiling), a, b,
-- ka, kb, n}.
CREATE OR REPLACE MACRO arm_cells_of(cell_list, a_jobs, b_jobs, overhead := false, lag_ceiling := @{lag_ceiling},
                                     lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    WITH c AS (FROM collapse_test_of(cell_list, a_jobs, b_jobs)),
    e AS (
        SELECT e.*, m.guard, m.guard_ord FROM effects_of(cell_list, a_jobs, b_jobs) e
        JOIN metric_defs m ON m.name = e.metric
        WHERE m.guard IS NOT NULL OR e.metric IN ('lag_p99_ms', 'wire/useful')),
    lags AS (
        SELECT cell, list_sort(list(a_value)) AS av, list_sort(list(b_value)) AS bv
        FROM pairs_of(a_jobs, b_jobs) WHERE metric = 'lag_p99_ms' AND list_contains(cell_list, cell) GROUP BY cell),
    mid AS (
        SELECT cell,
               CASE WHEN len(av) % 2 = 1 THEN av[len(av) // 2 + 1] ELSE (av[len(av) // 2] + av[len(av) // 2 + 1]) / 2 END AS a_mid,
               CASE WHEN len(bv) % 2 = 1 THEN bv[len(bv) // 2 + 1] ELSE (bv[len(bv) // 2] + bv[len(bv) // 2 + 1]) / 2 END AS b_mid
        FROM lags),
    brk AS (
        SELECT cell, 0 AS o, 0 AS mo, {'cell': cell, 'kind': 'collapsed', 'metric': 'collapsed', 'x': p, 'a': NULL::DOUBLE,
               'b': NULL::DOUBLE, 'ka': ka, 'kb': kb, 'n': n} AS br
        FROM c WHERE kb > ka AND p < 0.05
        UNION ALL
        SELECT cell, 1, 0, {'cell': cell, 'kind': 'lag_ceiling', 'metric': 'lag_p99_ms', 'x': lag_ceiling::DOUBLE, 'a': a_mid,
               'b': b_mid, 'ka': NULL::BIGINT, 'kb': NULL::BIGINT, 'n': NULL::BIGINT}
        FROM mid WHERE overhead AND a_mid < lag_ceiling AND lag_ceiling <= b_mid
        UNION ALL
        SELECT cell, 2, guard_ord, {'cell': cell, 'kind': 'worse', 'metric': metric, 'x': effect, 'a': NULL, 'b': NULL,
               'ka': NULL, 'kb': NULL, 'n': NULL}
        FROM e WHERE guard = 'worse' AND mark = '-'
        UNION ALL
        SELECT e.cell, 3, e.guard_ord, {'cell': e.cell, 'kind': 'guard', 'metric': e.metric, 'x': e.effect, 'a': NULL, 'b': NULL,
               'ka': NULL, 'kb': NULL, 'n': NULL}
        FROM e JOIN c USING (cell)
        WHERE e.guard = 'lo' AND c.ka = 0 AND e.effect IS NOT NULL
          AND e.lo > CASE WHEN overhead THEN lag_budget ELSE guard_ratio END
        UNION ALL
        SELECT cell, 4, guard_ord, {'cell': cell, 'kind': 'below_one', 'metric': metric, 'x': effect, 'a': NULL, 'b': NULL,
               'ka': NULL, 'kb': NULL, 'n': NULL}
        FROM e WHERE guard = 'hi' AND effect IS NOT NULL AND hi < 1),
    brks AS (SELECT cell, list(br ORDER BY o, mo) AS breaks FROM brk GROUP BY cell)
    SELECT c.cell, c.cell_ord, c.ka, c.kb, c.n, c.p, l.effect AS lag, l.lo AS lag_lo, l.hi AS lag_hi,
           w.effect AS wire, coalesce(b.breaks, []) AS breaks
    FROM c LEFT JOIN e l ON l.cell = c.cell AND l.metric = 'lag_p99_ms'
    LEFT JOIN e w ON w.cell = c.cell AND w.metric = 'wire/useful'
    LEFT JOIN brks b ON b.cell = c.cell;

-- An arm's score against the base over the cells (arm_cells_of): its
-- collapsed runs and the base's, the geometric means over cells of its
-- lag p99 ratio (p99) and its wire per useful byte ratio (wire), every
-- break, and rank_key, by which arms rank: broken ones last, then fewer
-- collapsed runs, then the lower p99 (wire when ranked for overhead).
CREATE OR REPLACE MACRO arm_score_of(cell_list, a_jobs, b_jobs, overhead := false, lag_ceiling := @{lag_ceiling},
                                     lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    WITH c AS (FROM arm_cells_of(cell_list, a_jobs, b_jobs, overhead := overhead, lag_ceiling := lag_ceiling,
                                 lag_budget := lag_budget, guard_ratio := guard_ratio)),
    t AS (
        SELECT coalesce(sum(kb), 0) AS collapsed, coalesce(sum(ka), 0) AS base_collapsed,
               exp(list_reduce(list(ln(lag) ORDER BY cell_ord) FILTER (WHERE lag IS NOT NULL), lambda acc, y: acc + y)
                   / count(lag)) AS p99,
               exp(list_reduce(list(ln(wire) ORDER BY cell_ord) FILTER (WHERE wire IS NOT NULL), lambda acc, y: acc + y)
                   / count(wire)) AS wire,
               coalesce(flatten(list(breaks ORDER BY cell_ord)), []) AS breaks
        FROM c)
    SELECT collapsed, base_collapsed, p99, wire, breaks, len(breaks) > 0 AS broken, overhead::BOOLEAN AS overhead,
           {'broken': len(breaks) > 0, 'collapsed': collapsed,
            'by': coalesce(CASE WHEN overhead THEN wire ELSE p99 END, 'inf'::DOUBLE)} AS rank_key
    FROM t;

-- An experiment's arm as a side: its jobs (experiment_jobs), each run's
-- source its index line's flush, in its cells' order. An arm holding two
-- jobs at one cell and seed is an error.
CREATE OR REPLACE MACRO arm_side(exp_name, arm_name) AS (
    SELECT CASE WHEN count(DISTINCT {'cell': cell, 'seed': seed}) < count(*)
                THEN error('experiment ' || exp_name || '''s arm ' || arm_name || ' holds two jobs at one cell and seed')
                ELSE list({'cell': cell, 'seed': seed, 'k': k, 'src': flush} ORDER BY matrix, cell, seed) END
    FROM (SELECT DISTINCT cell, seed, k, flush, matrix FROM experiment_jobs
          WHERE exp_id = exp_name AND arm_label = arm_name));

-- The cells of exp_b that exp_a has too, in exp_b's order (its matrices').
CREATE OR REPLACE MACRO exp_cells(exp_a, exp_b) AS (
    SELECT coalesce(list(cell ORDER BY matrix, cell), []) FROM experiment_cells
    WHERE exp_id = exp_b AND cell IN (SELECT cell FROM experiment_cells WHERE exp_id = exp_a));

-- pairs_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO pairs(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM pairs_of(arm_side(exp_a, arm_a), arm_side(exp_b, arm_b)) WHERE list_contains(exp_cells(exp_a, exp_b), cell);
-- effects_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells: a scorecard's lines.
CREATE OR REPLACE MACRO effects(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM effects_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b));
-- collapse_test_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO collapse_test(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM collapse_test_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b));
-- traces_of over arm_b of exp_b against arm_a of exp_a, every cell they share.
CREATE OR REPLACE MACRO traces(exp_a, arm_a, exp_b, arm_b) AS TABLE
    FROM traces_of(arm_side(exp_a, arm_a), arm_side(exp_b, arm_b));
-- arm_cells_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO arm_cells(exp_a, arm_a, exp_b, arm_b, overhead := false, lag_ceiling := @{lag_ceiling},
                                  lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    FROM arm_cells_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b), overhead := overhead,
                      lag_ceiling := lag_ceiling, lag_budget := lag_budget, guard_ratio := guard_ratio);
-- arm_score_of over arm_b of exp_b against arm_a of exp_a, in exp_cells' cells.
CREATE OR REPLACE MACRO arm_score(exp_a, arm_a, exp_b, arm_b, overhead := false, lag_ceiling := @{lag_ceiling},
                                  lag_budget := 'inf'::DOUBLE, guard_ratio := @{guard_ratio}) AS TABLE
    FROM arm_score_of(exp_cells(exp_a, exp_b), arm_side(exp_a, arm_a), arm_side(exp_b, arm_b), overhead := overhead,
                      lag_ceiling := lag_ceiling, lag_budget := lag_budget, guard_ratio := guard_ratio);

-- The cells where every arm of experiment exp_name collapsed on every
-- seed it ran (verdicts.collapsed): no arm can be told from another there.
CREATE OR REPLACE MACRO structural(exp_name) AS TABLE
    SELECT e.cell, count(DISTINCT e.arm_label) AS arms, count(*) AS runs
    FROM experiment_jobs e JOIN verdicts v ON v.k = e.k AND v.src = e.src
    WHERE e.exp_id = exp_name
    GROUP BY e.cell HAVING bool_and(v.collapsed)
    ORDER BY min(e.matrix), e.cell;

-- The honest viewers' lag each second of the runs `jobs`, a list of
-- structs with a job's `k` (its tables its own source's, jobs.src; other
-- fields left alone, so a side serves): k, src, viewer, sec, lag_ms,
-- frames and arrive_ms (NULL for a viewer there from the start). lag_ms is
-- viewer_seconds', carried from the viewer's last second with a read
-- +1000 ms a second without one, as analyze.py's lags() does: NULL before
-- its first read.
CREATE OR REPLACE MACRO viewer_lag_of(jobs) AS TABLE
    WITH j AS (
        SELECT DISTINCT b.k, b.src FROM (SELECT unnest(jobs, recursive := true)) u JOIN jobs b ON b.k = u.k),
    vs AS (
        SELECT s.k, s.src, s.viewer, s.sec, s.lag_ms, s.frames, w.arrive_ms
        FROM viewer_seconds s SEMI JOIN j USING (k, src)
        JOIN viewers w USING (k, src, viewer) WHERE NOT w.hostile)
    SELECT k, src, viewer, sec,
           coalesce(lag_ms, last_value(lag_ms IGNORE NULLS) OVER w
                    + 1000 * (sec - last_value(CASE WHEN lag_ms IS NOT NULL THEN sec END IGNORE NULLS) OVER w))
               AS lag_ms,
           frames, arrive_ms
    FROM vs WINDOW w AS (PARTITION BY k, src, viewer ORDER BY sec ROWS UNBOUNDED PRECEDING);

-- Each arm and cell of experiment exp_name, in the terms that tell a
-- startup tail from a steady spike:
--   runs, collapsed          its runs and the collapsed ones;
--   lag_p99_<phase>_ms       the p99 of the honest viewers' lag over their
--                            seconds of each phase (startup, steady,
--                            after_down, after_up), pooled over the runs:
--                            viewer_lag_of's carried lag. A second's phase
--                            is the viewer's (viewer_phases: its own
--                            startup, then its verdict's);
--   spike_s_<D>              per run, the seconds past startup, the run's
--                            and its own, in which an honest viewer lags
--                            past D ms: a player D behind would stall then;
--   stalls_per_min_<D>,      the rows' playback at delay D (the mean over
--   stalled_share_<D>,       runs; NULL in rows from before the sims
--   startup_ms_<D>           played them);
--   unhealthy_max_s,         the longest unhealthy spell (episodes) of any
--   unhealthy_p50_s          run, and the median of each run's longest;
-- for D in @{delay_target_ms} and @{delay_max_ms} (analyze.py's
-- DELIVERY_TARGET_MS and DELIVERY_MAX_MS).
-- needs: viewer_phases
CREATE OR REPLACE MACRO dynamics(exp_name) AS TABLE
    WITH ej AS (
        SELECT DISTINCT cell, arm_label, k, src, matrix FROM experiment_jobs WHERE exp_id = exp_name),
    lags AS (
        FROM viewer_lag_of((SELECT list({'k': k}) FROM experiment_jobs WHERE exp_id = exp_name))),
    phased AS (
        SELECT c.*, p.phase
        FROM lags c JOIN viewer_phases p ON p.k = c.k AND p.src = c.src AND p.viewer = c.viewer
                                         AND c.sec >= p.start AND c.sec < p.stop),
    lagq AS (
        SELECT e.cell, e.arm_label,
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE p.phase = 'startup') AS lag_p99_startup_ms,
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE p.phase = 'steady') AS lag_p99_steady_ms,
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE p.phase = 'after_down') AS lag_p99_after_down_ms,
               quantile_cont(p.lag_ms, 0.99) FILTER (WHERE p.phase = 'after_up') AS lag_p99_after_up_ms
        FROM phased p JOIN ej e USING (k, src) GROUP BY ALL),
    spikes AS (
        SELECT k, src, count(DISTINCT sec) FILTER (WHERE lag_ms > @{delay_target_ms}) AS s1,
               count(DISTINCT sec) FILTER (WHERE lag_ms > @{delay_max_ms}) AS s2
        FROM phased WHERE phase <> 'startup' GROUP BY ALL),
    spells AS (
        SELECT k, src, max(len) AS longest FROM episodes WHERE kind = 'unhealthy' GROUP BY ALL),
    runs AS (
        SELECT e.cell, e.arm_label, any_value(e.matrix) AS matrix, count(*) AS runs,
               count(*) FILTER (WHERE v.collapsed) AS collapsed,
               sum(coalesce(sp.s1, 0)) / count(*) AS spike_s_@{delay_target_ms},
               sum(coalesce(sp.s2, 0)) / count(*) AS spike_s_@{delay_max_ms},
               avg(@{r.stalls_per_min_@{delay_target_ms}}) AS stalls_per_min_@{delay_target_ms},
               avg(@{r.stalls_per_min_@{delay_max_ms}}) AS stalls_per_min_@{delay_max_ms},
               avg(@{r.stalled_share_@{delay_target_ms}}) AS stalled_share_@{delay_target_ms},
               avg(@{r.stalled_share_@{delay_max_ms}}) AS stalled_share_@{delay_max_ms},
               avg(@{r.startup_ms_@{delay_target_ms}}) AS startup_ms_@{delay_target_ms},
               avg(@{r.startup_ms_@{delay_max_ms}}) AS startup_ms_@{delay_max_ms},
               coalesce(max(u.longest), 0) AS unhealthy_max_s, median(coalesce(u.longest, 0)) AS unhealthy_p50_s
        FROM ej e JOIN verdicts v USING (k, src) JOIN rows r USING (k, src)
        LEFT JOIN spikes sp USING (k, src) LEFT JOIN spells u USING (k, src)
        GROUP BY ALL)
    SELECT r.cell, r.arm_label, r.runs, r.collapsed, l.* EXCLUDE (cell, arm_label), r.* EXCLUDE (cell, arm_label,
           matrix, runs, collapsed)
    FROM runs r LEFT JOIN lagq l USING (cell, arm_label)
    ORDER BY r.matrix, r.cell, r.arm_label;

-- The honest viewers' lag (viewer_lag_of) against the seconds since each
-- scenario event of experiment exp_name's runs, from `before` seconds
-- before it to `after` after, by cell, arm and kind: cell, arm_label,
-- kind, rel_s, windows, viewer_s and lag_p50_ms, lag_p90_ms, lag_p99_ms.
-- kind is the event's current id (event_kinds' replaced_by, by the job's
-- sim, for a retired one), of class scenario; an event at t <= 0 sets the
-- run up and anchors nothing. A viewer's second sec is rel_s =
-- floor(sec + 0.5 - t) after an event at t s, as the verdict places a
-- second in a phase, for rel_s from -before to after - 1. A run-scoped
-- kind's event (event_kinds.scope) takes every honest viewer's seconds of
-- its run, a viewer-scoped kind's (a join) only its node's viewer's.
-- Every event is a window of its own (a run's events of one kind at one
-- time, of one viewer for a viewer-scoped kind, are one), so a second
-- within two windows of a kind counts in each. windows is how many
-- windows hold a viewer-second at rel_s, and viewer_s how many
-- viewer-seconds with a lag they hold there (the quantiles').
-- needs: events event_kinds.scope event_kinds.replaced_by jobs.sim viewers.node
CREATE OR REPLACE MACRO lag_since(exp_name, before := 10, after := 120) AS TABLE
    WITH ej AS (
        SELECT DISTINCT cell, arm_label, k, src, matrix FROM experiment_jobs WHERE exp_id = exp_name),
    kinds AS (
        SELECT o.kind AS id, o.sim, n.kind, n.scope = 'viewer' AS own
        FROM event_kinds o JOIN event_kinds n ON n.kind = coalesce(o.replaced_by, o.kind) AND n.sim = o.sim
        WHERE n.class = 'scenario'),
    anchors AS (
        SELECT row_number() OVER () AS w, * FROM (
            SELECT DISTINCT e.k, e.src, c.kind, c.own, e.t_ms / 1000 AS t, CASE WHEN c.own THEN v.viewer END AS viewer
            FROM events e SEMI JOIN ej USING (k, src) JOIN jobs j USING (k, src)
            JOIN kinds c ON c.id = e.kind AND c.sim = j.sim
            LEFT JOIN viewers v ON v.k = e.k AND v.src = e.src AND v.node = e.node
            WHERE e.t_ms > 0 AND (NOT c.own OR v.viewer IS NOT NULL))),
    secs AS (
        SELECT w, k, src, kind, own, viewer, t,
               unnest(range(ceil(t - before - 0.5)::BIGINT, ceil(t + after - 0.5)::BIGINT)) AS sec
        FROM anchors),
    lags AS (
        FROM viewer_lag_of((SELECT list({'k': k}) FROM experiment_jobs WHERE exp_id = exp_name))),
    hits AS (
        SELECT s.w, s.k, s.src, s.kind, floor(s.sec + 0.5 - s.t)::BIGINT AS rel_s, l.lag_ms
        FROM secs s JOIN lags l USING (k, src, sec) WHERE NOT s.own
        UNION ALL
        SELECT s.w, s.k, s.src, s.kind, floor(s.sec + 0.5 - s.t)::BIGINT AS rel_s, l.lag_ms
        FROM secs s JOIN lags l USING (k, src, viewer, sec) WHERE s.own)
    SELECT e.cell, e.arm_label, h.kind, h.rel_s, count(DISTINCT h.w) AS windows, count(h.lag_ms) AS viewer_s,
           quantile_cont(h.lag_ms, 0.5) AS lag_p50_ms, quantile_cont(h.lag_ms, 0.9) AS lag_p90_ms,
           quantile_cont(h.lag_ms, 0.99) AS lag_p99_ms
    FROM hits h JOIN ej e USING (k, src)
    GROUP BY e.cell, e.arm_label, h.kind, h.rel_s
    ORDER BY min(e.matrix), e.cell, e.arm_label, h.kind, h.rel_s;
